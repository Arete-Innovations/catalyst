use crate::middleware::*;
use crate::structs::*;
use rocket::http::{ContentType, Status};
use rocket::tokio::fs;
use std::path::PathBuf;

pub enum StorageBucket {
    UserFiles,
    SharedFiles,
}

fn bucket_base_path(bucket: &StorageBucket, user: &Users) -> PathBuf {
    match bucket {
        StorageBucket::UserFiles => PathBuf::from("private/user_files").join(user.id.to_string()),
        StorageBucket::SharedFiles => PathBuf::from("private/shared_files"),
    }
}

pub async fn serve_file(bucket_str: &str, filename: &str, jwt_token: &str) -> Result<(ContentType, Vec<u8>), Status> {
    let user = jwt_to_user(jwt_token)?;

    let bucket = match bucket_str {
        "user_files" => StorageBucket::UserFiles,
        "shared_files" => StorageBucket::SharedFiles,
        _ => return Err(Status::NotFound),
    };

    let base_path = bucket_base_path(&bucket, &user);
    let file_path = base_path.join(filename);

    if !file_path.starts_with(&base_path) {
        return Err(Status::Forbidden);
    }

    let file_bytes = fs::read(&file_path).await.map_err(|_| Status::NotFound)?;

    let ext = file_path.extension().and_then(|ext| ext.to_str()).unwrap_or("bin");
    let content_type = ContentType::from_extension(ext).unwrap_or(ContentType::Binary);

    Ok((content_type, file_bytes))
}
