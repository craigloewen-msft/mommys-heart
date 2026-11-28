pub mod legal_memo;

use crate::server::docx::DOCX_MIME;
use crate::server::sharepoint;

pub const BACKFILL_LIMIT: i64 = 50;

pub async fn file_case_document(
    case_id: &str,
    folder: &[&str],
    file_name: &str,
    bytes: Vec<u8>,
) -> Result<(), String> {
    let case_folder = sharepoint::sync::ensure_folder_ref(case_id).await?;
    let store = sharepoint::store()?;
    let mut parent = case_folder.item_id;
    for segment in folder {
        parent = store.ensure_folder(&parent, segment).await?.item_id;
    }
    store.upload(&parent, file_name, bytes, DOCX_MIME).await?;
    Ok(())
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
    legal_memo::backfill().await
}
