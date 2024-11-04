// This file is @generated by prost-build.
/// UploadRequest represents the request with profile bytes and description.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UploadRequest {
    /// pprof bytes of the profile to be uploaded.
    #[prost(bytes = "vec", tag = "1")]
    pub profile: ::prost::alloc::vec::Vec<u8>,
    /// Description of the profile.
    #[prost(string, tag = "2")]
    pub description: ::prost::alloc::string::String,
}
/// UploadResponse represents the response with the link that can be used to access the profile.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UploadResponse {
    /// id of the uploaded profile.
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    /// link that can be used to access the profile.
    #[prost(string, tag = "2")]
    pub link: ::prost::alloc::string::String,
}
/// QueryRequest represents the request with the id of the profile to be queried.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryRequest {
    /// id of the profile to be queried.
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    /// Type of the profile to be queried.
    #[prost(string, optional, tag = "2")]
    pub profile_type: ::core::option::Option<::prost::alloc::string::String>,
    /// report_type is the type of report to return
    #[prost(
        enumeration = "super::super::query::v1alpha1::query_request::ReportType",
        tag = "3"
    )]
    pub report_type: i32,
    /// filter_query is the query string to filter the profile samples
    #[deprecated]
    #[prost(string, optional, tag = "4")]
    pub filter_query: ::core::option::Option<::prost::alloc::string::String>,
    /// node_trim_threshold is the threshold % where the nodes with Value less than this will be removed from the report
    #[prost(float, optional, tag = "5")]
    pub node_trim_threshold: ::core::option::Option<f32>,
    /// which runtime frames to filter out, often interpreter frames like python or ruby are not super useful by default
    #[deprecated]
    #[prost(message, optional, tag = "6")]
    pub runtime_filter: ::core::option::Option<
        super::super::query::v1alpha1::RuntimeFilter,
    >,
    /// group_by indicates the fields to group by
    #[prost(message, optional, tag = "7")]
    pub group_by: ::core::option::Option<super::super::query::v1alpha1::GroupBy>,
    /// invert_call_stack inverts the call stacks in the flamegraph
    #[prost(bool, optional, tag = "8")]
    pub invert_call_stack: ::core::option::Option<bool>,
    /// filter is a varying set of filter to apply to the query
    #[prost(message, repeated, tag = "9")]
    pub filter: ::prost::alloc::vec::Vec<super::super::query::v1alpha1::Filter>,
}
/// ProfileTypesRequest represents the profile types request with the id of the profile to be queried.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProfileTypesRequest {
    /// id of the profile's types to be queried.
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
}
/// ProfileTypesResponse represents the response with the list of available profile types.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProfileTypesResponse {
    /// list of available profile types.
    #[prost(message, repeated, tag = "1")]
    pub types: ::prost::alloc::vec::Vec<super::super::query::v1alpha1::ProfileType>,
    /// description of the profile uploaded.
    #[prost(string, tag = "2")]
    pub description: ::prost::alloc::string::String,
}
/// QueryResponse is the returned report for the given query.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryResponse {
    /// total is the total number of samples shown in the report.
    #[prost(int64, tag = "5")]
    pub total: i64,
    /// filtered is the number of samples filtered out of the report.
    #[prost(int64, tag = "6")]
    pub filtered: i64,
    /// report is the generated report
    #[prost(oneof = "query_response::Report", tags = "1, 2, 3, 4, 7, 8, 9, 10")]
    pub report: ::core::option::Option<query_response::Report>,
}
/// Nested message and enum types in `QueryResponse`.
pub mod query_response {
    /// report is the generated report
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Report {
        /// flamegraph is a flamegraph representation of the report
        #[prost(message, tag = "1")]
        Flamegraph(super::super::super::query::v1alpha1::Flamegraph),
        /// pprof is a pprof profile as compressed bytes
        #[prost(bytes, tag = "2")]
        Pprof(::prost::alloc::vec::Vec<u8>),
        /// top is a top list representation of the report
        #[prost(message, tag = "3")]
        Top(super::super::super::query::v1alpha1::Top),
        /// callgraph is a callgraph nodes and edges representation of the report
        #[prost(message, tag = "4")]
        Callgraph(super::super::super::query::v1alpha1::Callgraph),
        /// flamegraph_arrow is a flamegraph encoded as a arrow record
        #[prost(message, tag = "7")]
        FlamegraphArrow(super::super::super::query::v1alpha1::FlamegraphArrow),
        /// source is the source report type result
        #[prost(message, tag = "8")]
        Source(super::super::super::query::v1alpha1::Source),
        /// table_arrow is a table encoded as a arrow record
        #[prost(message, tag = "9")]
        TableArrow(super::super::super::query::v1alpha1::TableArrow),
        /// profile_metadata contains metadata about the profile i.e. binaries, labels
        #[prost(message, tag = "10")]
        ProfileMetadata(super::super::super::query::v1alpha1::ProfileMetadata),
    }
}
/// Generated server implementations.
pub mod share_service_server {
    #![allow(
        unused_variables,
        dead_code,
        missing_docs,
        clippy::wildcard_imports,
        clippy::let_unit_value,
    )]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with ShareServiceServer.
    #[async_trait]
    pub trait ShareService: std::marker::Send + std::marker::Sync + 'static {
        /// Uploads the profile and returns the link that can be used to access it.
        async fn upload(
            &self,
            request: tonic::Request<super::UploadRequest>,
        ) -> std::result::Result<tonic::Response<super::UploadResponse>, tonic::Status>;
        /// Query performs a profile query
        async fn query(
            &self,
            request: tonic::Request<super::QueryRequest>,
        ) -> std::result::Result<tonic::Response<super::QueryResponse>, tonic::Status>;
        /// ProfileTypes returns the list of available profile types.
        async fn profile_types(
            &self,
            request: tonic::Request<super::ProfileTypesRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ProfileTypesResponse>,
            tonic::Status,
        >;
    }
    /// Service that exposes APIs for sharing profiles.
    #[derive(Debug)]
    pub struct ShareServiceServer<T> {
        inner: Arc<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    impl<T> ShareServiceServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for ShareServiceServer<T>
    where
        T: ShareService,
        B: Body + std::marker::Send + 'static,
        B::Error: Into<StdError> + std::marker::Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            match req.uri().path() {
                "/parca.share.v1alpha1.ShareService/Upload" => {
                    #[allow(non_camel_case_types)]
                    struct UploadSvc<T: ShareService>(pub Arc<T>);
                    impl<
                        T: ShareService,
                    > tonic::server::UnaryService<super::UploadRequest>
                    for UploadSvc<T> {
                        type Response = super::UploadResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::UploadRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ShareService>::upload(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = UploadSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/parca.share.v1alpha1.ShareService/Query" => {
                    #[allow(non_camel_case_types)]
                    struct QuerySvc<T: ShareService>(pub Arc<T>);
                    impl<
                        T: ShareService,
                    > tonic::server::UnaryService<super::QueryRequest> for QuerySvc<T> {
                        type Response = super::QueryResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::QueryRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ShareService>::query(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = QuerySvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/parca.share.v1alpha1.ShareService/ProfileTypes" => {
                    #[allow(non_camel_case_types)]
                    struct ProfileTypesSvc<T: ShareService>(pub Arc<T>);
                    impl<
                        T: ShareService,
                    > tonic::server::UnaryService<super::ProfileTypesRequest>
                    for ProfileTypesSvc<T> {
                        type Response = super::ProfileTypesResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ProfileTypesRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ShareService>::profile_types(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = ProfileTypesSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        let mut response = http::Response::new(empty_body());
                        let headers = response.headers_mut();
                        headers
                            .insert(
                                tonic::Status::GRPC_STATUS,
                                (tonic::Code::Unimplemented as i32).into(),
                            );
                        headers
                            .insert(
                                http::header::CONTENT_TYPE,
                                tonic::metadata::GRPC_CONTENT_TYPE,
                            );
                        Ok(response)
                    })
                }
            }
        }
    }
    impl<T> Clone for ShareServiceServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    /// Generated gRPC service name
    pub const SERVICE_NAME: &str = "parca.share.v1alpha1.ShareService";
    impl<T> tonic::server::NamedService for ShareServiceServer<T> {
        const NAME: &'static str = SERVICE_NAME;
    }
}
