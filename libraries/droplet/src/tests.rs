use std::path::PathBuf;

use crate::versions::create_backend_constructor;

#[tokio::test]
pub async fn test_7z_list() {
    let zip_path = "/home/decduck/Dev/droplet/assets/TheGame.zip";
    let mut backend = create_backend_constructor(&PathBuf::from(zip_path)).unwrap()().unwrap();
    let files = backend.list_files().await.unwrap();
    tokio::fs::write("./test.txt", format!("{:?}", files))
        .await
        .unwrap();
}
