use crate::server::docx::DOCX_MIME;
use crate::server::sharepoint;

pub const BACKFILL_LIMIT: i64 = 50;

pub async fn file_case_document(
    case_id: &str,
    folder: &[&str],
    file_name: &str,
    bytes: Vec<u8>,
) -> Result<String, String> {
    let case_folder = sharepoint::sync::ensure_folder_ref(case_id).await?;
    let store = sharepoint::store()?;
    let mut parent = case_folder.item_id;
    for segment in folder {
        parent = store.ensure_folder(&parent, segment).await?.item_id;
    }
    store.upload(&parent, file_name, bytes, DOCX_MIME).await?;
    store
        .list_children(&parent)
        .await?
        .into_iter()
        .find(|entry| entry.name == file_name && !entry.web_url.is_empty())
        .map(|entry| entry.web_url)
        .ok_or_else(|| format!("'{file_name}' was uploaded but its link could not be read back"))
}

pub fn start_filing_backfill() {
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
        if let Err(e) = backfill().await {
            tracing::warn!("case record filing backfill stopped: {e}");
        }
    });
}

async fn backfill() -> Result<(), String> {
    Ok(())
}
