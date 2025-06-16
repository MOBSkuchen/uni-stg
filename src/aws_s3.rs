use aws_sdk_s3::{Client, Config};
use aws_sdk_s3::config::http::HttpResponse;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::copy_object::{CopyObjectError, CopyObjectOutput};
use aws_sdk_s3::operation::create_bucket::CreateBucketError;
use aws_sdk_s3::operation::delete_bucket::{DeleteBucketError};
use aws_sdk_s3::operation::delete_object::DeleteObjectError;
use aws_sdk_s3::operation::get_bucket_location::GetBucketLocationError;
use aws_sdk_s3::operation::get_object::{GetObjectError, GetObjectOutput};
use aws_sdk_s3::operation::list_buckets::ListBucketsError;
use aws_sdk_s3::operation::put_object::{PutObjectError, PutObjectOutput};
use aws_sdk_s3::types::Bucket;
use crate::{ClientBucket, ClientError, ClientInterface, ClientObject, EmptyReqRes, ReqRes};

macro_rules! aws_error_enum_and_impls {
    (
        $enum_name:ident, $client_error_variant:ident, $base_error:ident, $response_ty:ty, {
            $($variant:ident => $error_ty:ty),* $(,)?
        }
    ) => {
        // Define the AWSError enum
        #[derive(Debug)]
        pub enum $enum_name {
            $(
                $variant($error_ty),
            )*
        }

        $(
            impl From<SdkError<$error_ty, $response_ty>> for $base_error {
                fn from(value: SdkError<$error_ty, $response_ty>) -> Self {
                    $base_error::$client_error_variant($enum_name::$variant(value.into_service_error()))
                }
            }
        )*
    };
}

aws_error_enum_and_impls!(
    AWSError,
    AWSClient,
    ClientError,
    HttpResponse,
    {
        GetObjErr => GetObjectError,
        DelBucErr => DeleteBucketError,
        DelObjErr => DeleteObjectError,
        CopObjErr => CopyObjectError,
        GetLocErr => GetBucketLocationError,
        CreObjErr => CreateBucketError,
        PutObjErr => PutObjectError,
        LstBucErr => ListBucketsError,
    }
);

pub struct AWSBucket {
    bucket_name: String,
    location: Option<String>
}

impl ClientBucket for AWSBucket {
    fn id(&self) -> String {
        self.bucket_name.clone()
    }

    fn name(&self) -> String {
        self.bucket_name.clone()
    }

    fn location(&self) -> Option<String> {
        self.location.clone()
    }
}

impl From<Bucket> for AWSBucket {
    fn from(value: Bucket) -> Self {
        AWSBucket {bucket_name: value.name.clone().unwrap(), location: value.bucket_region}
    }
}

pub struct AWSConfig {
    config: Config
}

pub struct AWSClient {
    client: Client
}

pub struct AWSObject {
    object: GetObjectOutput,
    bucket: String
}

pub struct AWSObjectPut {
    object: PutObjectOutput,
    bucket: String
}

pub struct AWSObjectCopy {
    object: CopyObjectOutput,
    bucket: String
}

impl ClientObject for AWSObjectPut {
    fn size(&self) -> u64 {
        self.object.size.map(|t| {t as u64}).unwrap()
    }

    fn bucket_name(&self) -> String {
        self.bucket.clone()
    }

    fn id(&self) -> String {
        self.object.e_tag.clone().unwrap()
    }

    fn name(&self) -> String {
        self.id()
    }

    fn content_type(&self) -> Option<String> {
        None
    }
}

impl ClientObject for AWSObject {
    fn size(&self) -> u64 {
        self.object.content_length.map(|t| {t as u64}).unwrap()
    }

    fn bucket_name(&self) -> String {
        self.bucket.clone()
    }

    fn id(&self) -> String {
        self.object.e_tag.clone().unwrap()
    }

    fn name(&self) -> String {
        self.id()
    }

    fn content_type(&self) -> Option<String> {
        self.object.content_type.clone()
    }
}

impl ClientInterface for AWSClient {
    async fn static_download_object(&self, bucket_name: String, object_name: String, starting: Option<u64>, ending: Option<u64>) -> ReqRes<Vec<u8>> {
        let range = match (starting, ending) {
            (Some(s), Some(e)) => Some(format!("bytes={}-{}", s, e)),
            (Some(s), None)    => Some(format!("bytes={}-", s)),
            (None, Some(e))    => Some(format!("bytes=-{}", e)),
            (None, None)       => None,
        };
        let mut builder = self.client.get_object().bucket(&bucket_name).if_match(object_name);
        Ok(if let Some(range) = range {
            builder.range(range).send().await?.body.collect().await.unwrap().to_vec()
        } else {
            builder.send().await?.body.collect().await.unwrap().to_vec()
        })
    }

    /// Uploads an object
    /// Note: The content type of the returned object will always return None
    async fn static_upload_object(&self, bucket_name: String, object_name: String, data: Vec<u8>) -> ReqRes<impl ClientObject> {
        let object = self.client.put_object().bucket(&bucket_name).key(object_name).body(data.into()).send().await?;
        Ok(AWSObjectPut {object, bucket: bucket_name})
    }

    /// AWS S3 provides no URL for uploading objects. An empty string is returned.
    async fn url_upload_object(&self, _: String, _: String) -> ReqRes<String> {
        Ok("".to_string())
    }

    /// Creates a download URL
    /// Note: I don't know if this is correct
    async fn url_download_object(&self, bucket_name: String, object_name: String) -> ReqRes<String> {
        Ok(format!("https://{bucket_name}.s3.amazonaws.com/{object_name}"))
    }

    async fn remove_bucket(&self, bucket: String) -> EmptyReqRes {
        self.client.delete_bucket().bucket(bucket).send().await?;
        Ok(())
    }

    async fn remove_object(&self, bucket_name: String, object_name: String) -> EmptyReqRes {
        self.client.delete_object().bucket(bucket_name).key(object_name).send().await?;
        Ok(())
    }

    async fn create_bucket(&self, bucket_name: String) -> ReqRes<impl ClientBucket> {
        let location = self.client.create_bucket().bucket(bucket_name).send().await?.location;
        Ok(AWSBucket {bucket_name, location})
    }

    /// Copy an object from one object of bucket to another
    /// Note: AWS-S3 only supports copying within the same bucket
    async fn copy_object(&self, src_bucket: String, src_object: String, dest_bucket: String, dest_object: String) -> ReqRes<impl ClientObject> {
        assert_eq!(src_bucket, dest_bucket, "Source and destination buckets must be the same on AWS-S3");
        self.client.copy_object().bucket(src_bucket).key(dest_object).copy_source(src_object).send().await?.copy_object_result.unwrap();
        self.get_object(dest_bucket, dest_object)
    }

    async fn list_buckets(&self, max_results: Option<u32>) -> ReqRes<Vec<impl ClientBucket>> {
        let builder = self.client.list_buckets();
        Ok((if let Some(max_results) = max_results {
            builder.max_buckets(max_results as i32).send().await
        } else {
            builder.send().await
        })?.buckets.and_then(|t| { t.into_iter().map(|t1| {t1.into()}).collect() }).unwrap())
    }

    async fn get_bucket(&self, bucket_name: String) -> ReqRes<impl ClientBucket> {
        Ok(AWSBucket {bucket_name, location: Some(self.client.get_bucket_location().bucket(bucket_name).send().await?.location_constraint.unwrap().as_str().to_string()) })
    }

    async fn get_object(&self, bucket_name: String, object_name: String) -> ReqRes<impl ClientObject> {
        let object = self.client.get_object().bucket(&bucket_name).if_match(object_name).send().await?;
        Ok(AWSObject {object, bucket: bucket_name})
    }

    async fn list_objects(&self, bucket_name: String, max_results: Option<u32>) -> ReqRes<Vec<impl ClientObject>> {
        let builder = self.client.list_objects().bucket(bucket_name);
        if let Some(max_results) = max_results {
            builder.max_keys(max_results as i32).send()
        } else {
            builder.send()
        }
    }
}