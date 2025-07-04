use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::client::google_cloud_auth::credentials::CredentialsFile;
use google_cloud_storage::http::buckets::Bucket;
use google_cloud_storage::http::buckets::delete::{DeleteBucketParam, DeleteBucketRequest};
use google_cloud_storage::http::buckets::get::GetBucketRequest;
use google_cloud_storage::http::buckets::insert::{BucketCreationConfig, InsertBucketRequest};
use google_cloud_storage::http::buckets::list::ListBucketsRequest;
use google_cloud_storage::http::Error;
use google_cloud_storage::http::error::ErrorResponseItem;
use google_cloud_storage::http::objects::copy::CopyObjectRequest;
use google_cloud_storage::http::objects::delete::DeleteObjectRequest;
use google_cloud_storage::http::objects::download::Range;
use google_cloud_storage::http::objects::get::GetObjectRequest;
use google_cloud_storage::http::objects::list::ListObjectsRequest;
use google_cloud_storage::http::objects::Object;
use google_cloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};
use google_cloud_storage::sign::{SignedURLError, SignedURLMethod, SignedURLOptions};
use crate::{ClientBucket, ClientError, ClientInterface, ClientObject, EmptyReqRes, ReqRes};

pub enum GoogleCloudError {
    HttpError(Error),
    GoogleCloudStorageError(Vec<ErrorResponseItem>),
    SignedURLError(SignedURLError)
}

impl From<Error> for GoogleCloudError {
    fn from(value: Error) -> Self {
        match value {
            Error::Response(e) => GoogleCloudError::GoogleCloudStorageError(e.errors),
            _ => GoogleCloudError::HttpError(value)
        }
    }
}

impl From<Error> for ClientError {
    fn from(value: Error) -> Self {
        ClientError::GoogleCloudClient(match value {
            Error::Response(e) => GoogleCloudError::GoogleCloudStorageError(e.errors),
            _ => GoogleCloudError::HttpError(value)
        })
    }
}

impl From<SignedURLError> for ClientError {
    fn from(value: SignedURLError) -> Self {
        ClientError::GoogleCloudClient(GoogleCloudError::SignedURLError(value))
    }
}

pub struct GoogleCloudConfig {
    config: ClientConfig
}

impl GoogleCloudConfig {
    pub fn anonymous(project_id: String) -> Self {
        Self {
            config: (ClientConfig { project_id: Some(project_id), ..Default::default()}).anonymous()
        }
    }

    pub async fn standard_auth(project_id: String) -> Self {
        Self {
            config: (ClientConfig { project_id: Some(project_id), ..Default::default()}).with_auth().await.unwrap()
        }
    }

    pub async fn from_file(project_id: String, path: String) -> Self {
        Self {
            config: (ClientConfig { project_id: Some(project_id), ..Default::default()}).with_credentials(CredentialsFile::new_from_file(path).await.unwrap()).await.unwrap()
        }
    }

    pub async fn from_str(project_id: String, s: &str) -> Self {
        Self {
            config: (ClientConfig { project_id: Some(project_id), ..Default::default()}).with_credentials(CredentialsFile::new_from_str(s).await.unwrap()).await.unwrap()
        }
    }
}

pub struct GoogleCloudObject {
    object: Object
}

impl ClientObject for GoogleCloudObject {
    fn size(&self) -> u64 {
        self.object.size as u64
    }

    fn bucket_name(&self) -> String {
        self.object.bucket.clone()
    }

    fn id(&self) -> String {
        self.object.id.clone()
    }

    fn name(&self) -> String {
        self.object.name.clone()
    }

    fn content_type(&self) -> Option<String> {
        self.object.content_type.clone()
    }
}

impl From<Object> for GoogleCloudObject {
    fn from(value: Object) -> Self {
        GoogleCloudObject {object: value}
    }
}

pub struct GoogleCloudBucket {
    bucket: Bucket
}

impl ClientBucket for GoogleCloudBucket {
    fn id(&self) -> String {
        self.bucket.id.clone()
    }

    fn name(&self) -> String {
        self.bucket.name.clone()
    }

    fn location(&self) -> Option<String> {
        Some(self.bucket.location.clone())
    }
}

impl From<Bucket> for GoogleCloudBucket {
    fn from(value: Bucket) -> Self {
        GoogleCloudBucket {bucket: value}
    }
}

pub struct GoogleCloud {
    client: Client,
    project_id: String
}

impl GoogleCloud {
    pub fn new(config: GoogleCloudConfig) -> Self {
        let project_id = config.config.project_id.clone().unwrap();
        let client = Client::new(config.config);
        Self { client, project_id }
    }
}

impl ClientInterface for GoogleCloud {
    async fn static_download_object(&self, bucket: String, object: String, starting: Option<u64>, ending: Option<u64>) -> ReqRes<Vec<u8>> {
        let req = GetObjectRequest {
            bucket,
            object,
            ..Default::default()
        };
        Ok(self.client.download_object(&req, &Range(starting, ending)).await?)
    }

    async fn static_upload_object(&self, bucket: String, object: String, data: Vec<u8>) -> ReqRes<GoogleCloudObject> {
        let upload_type = UploadType::Simple(Media::new(object));
        let req = UploadObjectRequest {
            bucket,
            ..Default::default()
        };
        Ok(self.client.upload_object(&req, data, &upload_type).await?.into())
    }

    async fn url_upload_object(&self, bucket: String, object: String) -> ReqRes<String> {
        Ok(self.client.signed_url(bucket.as_str(), object.as_str(), None, None, SignedURLOptions { method: SignedURLMethod::PUT, ..Default::default() }).await?)
    }

    async fn url_download_object(&self, bucket: String, object: String) -> ReqRes<String> {
        Ok(self.client.signed_url(bucket.as_str(), object.as_str(), None, None, SignedURLOptions::default()).await?)
    }

    async fn remove_bucket(&self, bucket: String) -> EmptyReqRes {
        let req = DeleteBucketRequest {
            bucket,
            param: DeleteBucketParam::default()
        };
        Ok(self.client.delete_bucket(&req).await?)
    }

    async fn remove_object(&self, bucket: String, object: String) -> EmptyReqRes {
        let req = DeleteObjectRequest {
            bucket,
            object,
            ..Default::default()
        };
        Ok(self.client.delete_object(&req).await?)
    }

    async fn create_bucket(&self, bucket: String) -> ReqRes<GoogleCloudBucket> {
        let req = InsertBucketRequest {
            name: bucket,
            param: Default::default(),
            bucket: BucketCreationConfig::default()
        };
        Ok(self.client.insert_bucket(&req).await?.into())
    }

    async fn copy_object(&self, src_bucket: String, src_object: String, dest_bucket: String, dest_object: String) -> ReqRes<GoogleCloudObject> {
        let req = CopyObjectRequest {
            destination_bucket: dest_bucket,
            destination_object: dest_object,
            source_object: src_object,
            source_bucket: src_bucket,
            ..Default::default()
        };
        Ok(GoogleCloudObject::from(self.client.copy_object(&req).await?))
    }

    async fn list_buckets(&self, max_results: Option<u32>) -> ReqRes<Vec<GoogleCloudBucket>> {
        let req = ListBucketsRequest {
            project: self.project_id.clone(),
            max_results: max_results.map(|t| t as i32),
            ..Default::default()
        };
        Ok(self.client.list_buckets(&req).await?.items.into_iter().map(|x| {x.into()}).collect())
    }

    async fn get_bucket(&self, bucket_name: String) -> ReqRes<GoogleCloudBucket> {
        let req = GetBucketRequest {
            bucket: bucket_name,
            ..Default::default()
        };
        Ok(self.client.get_bucket(&req).await?.into())
    }

    async fn get_object(&self, bucket_name: String, object_name: String) -> ReqRes<GoogleCloudObject> {
        let req = GetObjectRequest {
            bucket: bucket_name,
            object: object_name,
            ..Default::default()
        };
        Ok(self.client.get_object(&req).await?.into())
    }

    async fn list_objects(&self, bucket: String, max_results: Option<u32>) -> ReqRes<Vec<GoogleCloudObject>> {
        let req = ListObjectsRequest {
            bucket,
            max_results: max_results.map(|t| t as i32),
            ..Default::default()
        };
        Ok(self.client.list_objects(&req).await?.items.unwrap().into_iter().map(|x| {x.into()}).collect())
    }
}