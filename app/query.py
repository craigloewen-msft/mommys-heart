from openai import AzureOpenAI
import chromadb

from app.config import settings
from app.viewer import slugify

client = AzureOpenAI(
    azure_endpoint=settings.AZURE_OPENAI_ENDPOINT,
    api_key=settings.AZURE_OPENAI_API_KEY,
    api_version=settings.AZURE_OPENAI_API_VERSION,
)

SYSTEM_PROMPT = """You are a helpful document assistant. Answer the user's question using ONLY the provided context from the source documents. Follow these rules strictly:

1. Base your answer exclusively on the provided context. Do not use outside knowledge.
2. If the context does not contain enough information to answer, say so clearly.
3. After your answer, list the sources you used in this exact format:
   📄 **Source:** [filename] — Section: [heading/section name]
4. Quote or closely paraphrase the relevant text when it strengthens your answer.
5. Be concise but thorough."""


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
    """Run the full RAG pipeline: embed → retrieve → generate answer."""
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
    )

    answer = response.choices[0].message.content

    # Build deduplicated source list
    seen = set()
    sources = []
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
    }
