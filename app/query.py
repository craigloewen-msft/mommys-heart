import json

from openai import AzureOpenAI
import chromadb

from app.config import settings
from app.viewer import slugify

client = AzureOpenAI(
    azure_endpoint=settings.AZURE_OPENAI_ENDPOINT,
    api_key=settings.AZURE_OPENAI_API_KEY,
    api_version=settings.AZURE_OPENAI_API_VERSION,
)

SYSTEM_PROMPT = """You are the Mommy's Heart AI Assistant. You are given numbered context excerpts retrieved from Mommy's Heart's own documents. They may or may not be relevant to the user's question.

Answer the user's question following these rules:
1. PREFER the provided document context. If one or more excerpts actually contain the answer, base your answer on them.
2. If the excerpts do NOT contain the answer (even though they were retrieved), DO still answer the question directly using your own general knowledge. Do not refuse or say "the documents don't contain this" — just answer helpfully from what you know.
3. Only say you cannot help if you genuinely do not know the answer and it is not in the excerpts.
4. Be concise but thorough. Do NOT include any source list, citations, or "Sources" section in your answer text — sources are handled separately by the application.

Respond with a JSON object containing exactly these fields:
- "answer": your answer as plain text, with no source list.
- "used_sources": an array of the integer numbers of the excerpts you ACTUALLY used to write the answer (e.g. [1, 3]). Use an empty array [] if you did not use any excerpt (for example when answering from general knowledge or when the documents are irrelevant). Never list an excerpt you did not actually rely on.
- "source_type": one of:
    - "documents"          → your answer came entirely from the listed excerpts.
    - "general_knowledge"  → you used your own general knowledge (used_sources must be []).
    - "mixed"              → you combined listed excerpts with your own general knowledge.
"""


def _embed_query(text: str) -> list[float]:
    response = client.embeddings.create(
        input=[text],
        model=settings.AZURE_OPENAI_EMBEDDING_DEPLOYMENT,
    )
    return response.data[0].embedding


def _retrieve_context(query_embedding: list[float], top_k: int = 5) -> list[dict]:
    chroma_client = chromadb.PersistentClient(path=settings.CHROMA_DIR)
    collection = chroma_client.get_collection("documents")

    results = collection.query(
        query_embeddings=[query_embedding],
        n_results=top_k,
        include=["documents", "metadatas", "distances"],
    )

    contexts = []
    for i in range(len(results["ids"][0])):
        contexts.append({
            "text": results["documents"][0][i],
            "filename": results["metadatas"][0][i]["filename"],
            "heading": results["metadatas"][0][i]["heading"],
            "relevance_score": 1 - results["distances"][0][i],  # cosine similarity
        })
    return contexts


def query_documents(question: str) -> dict:
    """Run the full RAG pipeline: embed → retrieve → generate answer.

    The model answers from the retrieved documents when possible, and may fall
    back to its own general knowledge otherwise. The returned ``source_type``
    tells the caller which happened so the UI can clearly distinguish answers
    grounded in the documents from answers based on general knowledge.
    """
    query_embedding = _embed_query(question)
    contexts = _retrieve_context(query_embedding)

    # Build context block for the LLM
    context_block = ""
    for i, ctx in enumerate(contexts, 1):
        context_block += (
            f"\n--- Source {i}: {ctx['filename']} | Section: {ctx['heading']} ---\n"
            f"{ctx['text']}\n"
        )

    response = client.chat.completions.create(
        model=settings.AZURE_OPENAI_CHAT_DEPLOYMENT,
        messages=[
            {"role": "system", "content": SYSTEM_PROMPT},
            {
                "role": "user",
                "content": f"Context from documents:\n{context_block}\n\nQuestion: {question}",
            },
        ],
        temperature=0.3,
        max_tokens=1500,
        response_format={"type": "json_object"},
    )

    raw = response.choices[0].message.content or "{}"
    used_indices: list[int] = []
    try:
        parsed = json.loads(raw)
        answer = parsed.get("answer") or "(no answer)"
        source_type = parsed.get("source_type", "documents")
        raw_used = parsed.get("used_sources", [])
        if isinstance(raw_used, list):
            for v in raw_used:
                try:
                    used_indices.append(int(v))
                except (ValueError, TypeError):
                    continue
    except (json.JSONDecodeError, AttributeError):
        # Fall back to treating the whole response as the answer
        answer = raw
        source_type = "general_knowledge"

    if source_type not in ("documents", "general_knowledge", "mixed"):
        source_type = "documents"

    # Map the model's 1-based excerpt numbers back to retrieved contexts, keeping
    # only valid, in-range indices. This ensures we ONLY show sources the model
    # actually used — not every chunk that happened to be retrieved.
    used_contexts = [
        contexts[i - 1] for i in used_indices if 1 <= i <= len(contexts)
    ]

    # Reconcile the declared mode with what was actually cited so the badge and
    # the source list can never contradict each other.
    if not used_contexts:
        # Nothing was genuinely cited → this is not a document-grounded answer.
        if source_type in ("documents", "mixed"):
            source_type = "general_knowledge"
    elif source_type == "general_knowledge":
        # Cited excerpts but claimed general knowledge → it's at least mixed.
        source_type = "mixed"

    # Build the (deduplicated) list of sources that were actually used.
    sources = []
    seen = set()
    for ctx in used_contexts:
        key = (ctx["filename"], ctx["heading"])
        if key in seen:
            continue
        seen.add(key)
        first_heading = ctx["heading"].split(",")[0].strip()
        sources.append({
            "filename": ctx["filename"],
            "heading": ctx["heading"],
            "snippet": ctx["text"][:200] + "..." if len(ctx["text"]) > 200 else ctx["text"],
            "relevance": round(ctx["relevance_score"], 3),
            "anchor": slugify(first_heading),
        })

    return {
        "answer": answer,
        "sources": sources,
        "source_type": source_type,
    }
