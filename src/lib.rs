#[cfg(feature = "aws_s3")]
use crate::aws_s3::AWSError;
#[cfg(feature = "aws_s3")]
mod aws_s3;

#[cfg(feature = "google_cloud")]
use crate::google_cloud::GoogleCloudError;
#[cfg(feature = "google_cloud")]
mod google_cloud;


// TODO: Find a better way for async traits

#[allow(async_fn_in_trait)]
pub trait ClientInterface {
    /// Statically (at once) downloads an object from remote
    async fn static_download_object(&self, bucket: String, object_id: String, starting: Option<u64>, ending: Option<u64>) -> ReqRes<Vec<u8>>;
    /// Statically (at once) uploads an object to remote
    async fn static_upload_object(&self, bucket: String, object_id: String, data: Vec<u8>) -> ReqRes<impl ClientObject>;
    /// Gets a URL which can be used to upload data
    /// Not supported: AWS-S3
    async fn url_upload_object(&self, bucket: String, object_id: String) -> ReqRes<String>;
    /// Gets a URL which can be used to download data
    async fn url_download_object(&self, bucket: String, object_id: String) -> ReqRes<String>;
    /// Deletes a bucket
    async fn remove_bucket(&self, bucket: String) -> EmptyReqRes;
    /// Deletes an object from a bucket
    async fn remove_object(&self, bucket: String, object_id: String) -> EmptyReqRes;
    /// Creates a new bucket
    async fn create_bucket(&self, bucket: String) -> ReqRes<impl ClientBucket>;
    /// Copies an object from one position to another
    /// Varies (see implementation): AWS-S3
    async fn copy_object(&self, src_bucket: String, src_object: String, dest_bucket: String, dest_object: String) -> ReqRes<impl ClientObject>;
    /// List available buckets
    async fn list_buckets(&self, max_results: Option<u32>) -> ReqRes<Vec<impl ClientBucket>>;
    /// Get a specific bucket
    async fn get_bucket(&self, bucket_name: String) -> ReqRes<impl ClientBucket>;
    /// Get a specific object from a bucket
    async fn get_object(&self, bucket_name: String, object_name: String) -> ReqRes<impl ClientBucket>;
    /// List objects in a bucket
    async fn list_objects(&self, bucket_name: String, max_results: Option<u32>) -> ReqRes<Vec<impl ClientObject>>;
}

#[allow(async_fn_in_trait)]
pub trait ClientObject {
    /// Returns the total byte-size of the object
    fn size(&self) -> u64;
    /// Name of the bucket the object is in
    fn bucket_name(&self) -> String;
    /// ID of the object (often etag; often same as name)
    fn id(&self) -> String;
    /// Name of the object (often etag; often same as name)
    fn name(&self) -> String;
    /// Object's content type (if available)
    fn content_type(&self) -> Option<String>;
}

#[allow(async_fn_in_trait)]
pub trait ClientBucket {
    /// ID of the bucket (often etag; often same as name)
    fn id(&self) -> String;
    /// Name of the bucket (often etag; often same as name)
    fn name(&self) -> String;
    /// Location of the bucket (example: 'us-west1')
    fn location(&self) -> Option<String>;
}

/// A wrapper around errors from different clients
/// TODO: Create a unified Access point
pub enum ClientError {
    #[cfg(feature = "google_cloud")]
    GoogleCloudClient(GoogleCloudError),
    #[cfg(feature = "aws_s3")]
    AWSClient(AWSError)
}

pub type ReqRes<T> = Result<T, ClientError>;
pub type EmptyReqRes = Result<(), ClientError>;