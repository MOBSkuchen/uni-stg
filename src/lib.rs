use crate::google_cloud::google_cloud::GoogleCloudError;

mod google_cloud;

// TODO: Add bucket interface

pub trait ClientInterface {
    async fn static_download_object(&self, bucket: String, object_id: String, starting: Option<u64>, ending: Option<u64>) -> ReqRes<Vec<u8>>;
    async fn static_upload_object(&mut self, bucket: String, object_id: String, data: Vec<u8>) -> ReqRes<impl ClientObject>;
    async fn url_upload_object(&mut self, bucket: String, object_id: String) -> ReqRes<String>;
    async fn url_download_object(&self, bucket: String, object_id: String) -> ReqRes<String>;
    async fn remove_bucket(&self, bucket: String) -> EmptyReqRes;
    async fn remove_object(&self, bucket: String, object_id: String) -> EmptyReqRes;
    async fn create_bucket(&self, bucket: String) -> EmptyReqRes;
    async fn copy_object(&self, src_bucket: String, src_object: String, dest_bucket: String, dest_object: String) -> ReqRes<impl ClientObject>;
    async fn list_buckets(&self, project: String, max_results: Option<u32>) -> ReqRes<Vec<String>>;
    async fn list_objects(&self, bucket: String, max_results: Option<u32>) -> ReqRes<Vec<impl ClientObject>>;
}

pub trait ClientObject {
    async fn size(&self) -> u64;
    async fn bucket(&self) -> String;
    async fn id(&self) -> String;
    async fn name(&self) -> String;
    async fn content_type(&self) -> Option<String>;
}

pub enum ClientError {
    #[cfg(feature = "google_cloud")]
    GoogleCloudClient(GoogleCloudError)
}

pub type ReqRes<T> = Result<T, ClientError>;
pub type EmptyReqRes = Result<(), ClientError>;