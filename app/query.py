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

SYSTEM_PROMPT = """You are the Mommy's Heart AI Assistant. You are given context excerpts retrieved from Mommy's Heart's own documents.

Answer the user's question following these rules:
1. PREFER the provided document context. If it contains the answer, base your answer on it.
2. If the document context does NOT contain enough information to answer, you MAY answer using your own general knowledge — but only if you are confident the information is accurate and helpful.
3. If you cannot answer from either the documents or your own knowledge, say so honestly.
4. Be concise but thorough. Do NOT include a source list in your answer text; sources are handled separately.

Respond with a JSON object containing exactly these fields:
- "answer": your answer as plain text.
- "source_type": one of:
    - "documents"          → your answer comes entirely from the provided document context.
    - "general_knowledge"  → the documents did not cover this, so you used your own general knowledge.
    - "mixed"              → you combined the provided documents with your own general knowledge.
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
    try:
        parsed = json.loads(raw)
        answer = parsed.get("answer") or "(no answer)"
        source_type = parsed.get("source_type", "documents")
    except (json.JSONDecodeError, AttributeError):
        # Fall back to treating the whole response as the answer
        answer = raw
        source_type = "documents"

    if source_type not in ("documents", "general_knowledge", "mixed"):
        source_type = "documents"

    # Only surface document source cards when the documents were actually used.
    sources = []
    if source_type in ("documents", "mixed"):
        seen = set()
        for ctx in contexts:
            key = (ctx["filename"], ctx["heading"])
            if key not in seen:
                seen.add(key)
                # Use the first heading for the anchor link
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
