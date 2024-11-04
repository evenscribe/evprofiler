// This file is @generated by prost-build.
/// TargetsRequest contains the parameters for the set of targets to return
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct TargetsRequest {
    /// state is the state of targets to returns
    #[prost(enumeration = "targets_request::State", tag = "1")]
    pub state: i32,
}
/// Nested message and enum types in `TargetsRequest`.
pub mod targets_request {
    /// State represents the current state of a target
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum State {
        /// STATE_ANY_UNSPECIFIED unspecified
        AnyUnspecified = 0,
        /// STATE_ACTIVE target active state
        Active = 1,
        /// STATE_DROPPED target dropped state
        Dropped = 2,
    }
    impl State {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Self::AnyUnspecified => "STATE_ANY_UNSPECIFIED",
                Self::Active => "STATE_ACTIVE",
                Self::Dropped => "STATE_DROPPED",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "STATE_ANY_UNSPECIFIED" => Some(Self::AnyUnspecified),
                "STATE_ACTIVE" => Some(Self::Active),
                "STATE_DROPPED" => Some(Self::Dropped),
                _ => None,
            }
        }
    }
}
/// TargetsResponse is the set of targets for the given requested state
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TargetsResponse {
    /// targets is the mapping of targets
    #[prost(map = "string, message", tag = "1")]
    pub targets: ::std::collections::HashMap<::prost::alloc::string::String, Targets>,
}
/// Targets is a list of targets
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Targets {
    /// targets is a list of targets
    #[prost(message, repeated, tag = "1")]
    pub targets: ::prost::alloc::vec::Vec<Target>,
}
/// Target is the scrape target representation
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Target {
    /// discovered_labels are the set of labels for the target that have been discovered
    #[prost(message, optional, tag = "1")]
    pub discovered_labels: ::core::option::Option<
        super::super::profilestore::v1alpha1::LabelSet,
    >,
    /// labels are the set of labels given for the target
    #[prost(message, optional, tag = "2")]
    pub labels: ::core::option::Option<super::super::profilestore::v1alpha1::LabelSet>,
    /// last_error is the error message most recently received from a scrape attempt
    #[prost(string, tag = "3")]
    pub last_error: ::prost::alloc::string::String,
    /// last_scrape is the time stamp the last scrape request was performed
    #[prost(message, optional, tag = "4")]
    pub last_scrape: ::core::option::Option<::prost_types::Timestamp>,
    /// last_scrape_duration is the duration of the last scrape request
    #[prost(message, optional, tag = "5")]
    pub last_scrape_duration: ::core::option::Option<::prost_types::Duration>,
    /// url is the url of the target
    #[prost(string, tag = "6")]
    pub url: ::prost::alloc::string::String,
    /// health indicates the current health of the target
    #[prost(enumeration = "target::Health", tag = "7")]
    pub health: i32,
}
/// Nested message and enum types in `Target`.
pub mod target {
    /// Health are the possible health values of a target
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum Health {
        /// HEALTH_UNKNOWN_UNSPECIFIED unspecified
        UnknownUnspecified = 0,
        /// HEALTH_GOOD healthy target
        Good = 1,
        /// HEALTH_BAD unhealthy target
        Bad = 2,
    }
    impl Health {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Self::UnknownUnspecified => "HEALTH_UNKNOWN_UNSPECIFIED",
                Self::Good => "HEALTH_GOOD",
                Self::Bad => "HEALTH_BAD",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "HEALTH_UNKNOWN_UNSPECIFIED" => Some(Self::UnknownUnspecified),
                "HEALTH_GOOD" => Some(Self::Good),
                "HEALTH_BAD" => Some(Self::Bad),
                _ => None,
            }
        }
    }
}
/// Generated server implementations.
pub mod scrape_service_server {
    #![allow(
        unused_variables,
        dead_code,
        missing_docs,
        clippy::wildcard_imports,
        clippy::let_unit_value,
    )]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with ScrapeServiceServer.
    #[async_trait]
    pub trait ScrapeService: std::marker::Send + std::marker::Sync + 'static {
        /// Targets returns the set of scrape targets that are configured
        async fn targets(
            &self,
            request: tonic::Request<super::TargetsRequest>,
        ) -> std::result::Result<tonic::Response<super::TargetsResponse>, tonic::Status>;
    }
    /// ScrapeService maintains the set of scrape targets
    #[derive(Debug)]
    pub struct ScrapeServiceServer<T> {
        inner: Arc<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    impl<T> ScrapeServiceServer<T> {
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
    impl<T, B> tonic::codegen::Service<http::Request<B>> for ScrapeServiceServer<T>
    where
        T: ScrapeService,
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
                "/parca.scrape.v1alpha1.ScrapeService/Targets" => {
                    #[allow(non_camel_case_types)]
                    struct TargetsSvc<T: ScrapeService>(pub Arc<T>);
                    impl<
                        T: ScrapeService,
                    > tonic::server::UnaryService<super::TargetsRequest>
                    for TargetsSvc<T> {
                        type Response = super::TargetsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::TargetsRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ScrapeService>::targets(&inner, request).await
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
                        let method = TargetsSvc(inner);
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
    impl<T> Clone for ScrapeServiceServer<T> {
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
    pub const SERVICE_NAME: &str = "parca.scrape.v1alpha1.ScrapeService";
    impl<T> tonic::server::NamedService for ScrapeServiceServer<T> {
        const NAME: &'static str = SERVICE_NAME;
    }
}
