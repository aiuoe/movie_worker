//! Cliente S3 minimalista — sin SDK, sólo reqwest + hmac/sha2.
//!
//! Soporta MinIO, AWS S3, R2, Wasabi, B2 — cualquier backend que hable el
//! protocolo S3 estándar. La diferencia entre backends es sólo:
//!   - endpoint URL
//!   - region
//!   - signing: header-based (PUT/GET) vs query-based (presigned URLs)
//!
//! SigV4 lo implementamos a mano (~80 líneas). Mantenerlo in-house evita
//! el costo binario del AWS SDK oficial (~30MB) y los problemas de version
//! matching con rustc.

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::time::Duration;

use super::{Backend, ObjectInfo, Storage};

type HmacSha256 = Hmac<Sha256>;

pub struct S3Storage {
    client: Client,
    endpoint: String,
    public_endpoint: String,
    region: String,
    bucket: String,
    access_key: String,
    secret_key: String,
    path_style: bool,
    backend: Backend,
}

impl S3Storage {
    pub fn new(
        backend: Backend,
        endpoint: Option<String>,
        public_endpoint: Option<String>,
        bucket: String,
        region: String,
        access_key: String,
        secret_key: String,
        path_style: bool,
    ) -> Result<Self> {
        let endpoint = endpoint.ok_or_else(|| anyhow!("S3_ENDPOINT required"))?;
        let public_ep = public_endpoint.unwrap_or_else(|| endpoint.clone());

        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .context("reqwest client")?;

        Ok(Self {
            client,
            endpoint: endpoint.trim_end_matches('/').to_string(),
            public_endpoint: public_ep.trim_end_matches('/').to_string(),
            region,
            bucket,
            access_key,
            secret_key,
            path_style,
            backend,
        })
    }

    fn host_for(&self, endpoint: &str, key: &str) -> String {
        if self.path_style {
            // http://host:9000/bucket/key
            format!("{}/{}/{}", endpoint.trim_end_matches('/'), self.bucket, key)
        } else {
            // https://bucket.s3.region.amazonaws.com/key
            format!("{}/{}/{}", endpoint, self.bucket, key)
        }
    }

    /// Host con puerto (ej "minio:9000"). El signature canonical host
    /// tiene que matchear EXACTAMENTE al header Host HTTP — si no, 403.
    fn host_header_value(&self, endpoint: &str) -> String {
        let u = url::Url::parse(endpoint).expect("valid endpoint");
        let h = u.host_str().unwrap_or("localhost");
        match u.port() {
            Some(p) => format!("{h}:{p}"),
            None => h.to_string(),
        }
    }

    /// SigV4 header auth — para PUT, GET, LIST.
    fn sign_request(
        &self,
        method: &str,
        url_path: &str,
        query: &str,
        body_hash: &str,
        content_type: Option<&str>,
    ) -> HeaderMap {
        let now = Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();

        let canonical_headers = match content_type {
            Some(ct) => format!("content-type:{ct}\nhost:{url_path}\nx-amz-content-sha256:{body_hash}\nx-amz-date:{amz_date}\n"),
            None => format!("host:{url_path}\nx-amz-content-sha256:{body_hash}\nx-amz-date:{amz_date}\n"),
        };
        let signed_headers = match content_type {
            Some(_) => "content-type;host;x-amz-content-sha256;x-amz-date",
            None => "host;x-amz-content-sha256;x-amz-date",
        };

        let canonical_request = format!(
            "{method}\n{url_path}\n{query}\n{canonical_headers}\n{signed_headers}\n{body_hash}",
        );
        let cr_hash = sha256_hex(canonical_request.as_bytes());

        let credential_scope = format!("{date_stamp}/{}/{}/aws4_request", self.region, "s3");
        let string_to_sign =
            format!("AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{cr_hash}");

        let signing_key = derive_key(&self.secret_key, &date_stamp, &self.region, "s3");
        let signature = {
            let mut mac = HmacSha256::new_from_slice(&signing_key).unwrap();
            mac.update(string_to_sign.as_bytes());
            hex::encode(mac.finalize().into_bytes())
        };

        let auth = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.access_key, credential_scope, signed_headers, signature
        );

        let mut h = HeaderMap::new();
        h.insert(
            HeaderName::from_static("x-amz-date"),
            HeaderValue::from_str(&amz_date).unwrap(),
        );
        h.insert(
            HeaderName::from_static("x-amz-content-sha256"),
            HeaderValue::from_str(body_hash).unwrap(),
        );
        h.insert(AUTHORIZATION, HeaderValue::from_str(&auth).unwrap());
        if let Some(ct) = content_type {
            h.insert(CONTENT_TYPE, HeaderValue::from_str(ct).unwrap());
        }
        h
    }
}

fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}

fn derive_key(secret: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
    let k = format!("AWS4{secret}");
    let k_date = hmac_sha256(k.as_bytes(), date.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, b"aws4_request")
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).unwrap();
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

#[async_trait]
impl Storage for S3Storage {
    fn backend(&self) -> Backend {
        self.backend
    }

    fn bucket(&self) -> &str {
        &self.bucket
    }

    async fn ping(&self) -> Result<()> {
        let url = format!("{}/{}", self.endpoint.trim_end_matches('/'), self.bucket);
        let host = self.host_header_value(&self.endpoint);
        let canonical_uri = format!("/{}", self.bucket);
        let body_hash = sha256_hex(b"");
        let mut headers = self.sign_request("HEAD", &canonical_uri, &host, "", &body_hash, None);
        headers.insert(reqwest::header::HOST, HeaderValue::from_str(&host).unwrap());

        let resp = self
            .client
            .head(&url)
            .headers(headers)
            .send()
            .await
            .context("HEAD bucket")?;
        let status = resp.status();
        if status.is_success() || status.as_u16() == 404 {
            Ok(())
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(anyhow!("ping status {}: {}", status, body))
        }
    }

    async fn put(&self, key: &str, data: Bytes, content_type: Option<&str>) -> Result<()> {
        let ct = content_type.unwrap_or("application/octet-stream");
        let body_hash = sha256_hex(&data);
        let url = self.host_for(&self.endpoint, key);
        let host = self.host_header_value(&self.endpoint);
        let canonical_uri = format!("/{}/{}", self.bucket, key);

        let mut headers = self.sign_request("PUT", &canonical_uri, &host, "", &body_hash, Some(ct));
        headers.insert(reqwest::header::HOST, HeaderValue::from_str(&host).unwrap());

        let resp = self
            .client
            .put(&url)
            .headers(headers)
            .body(data.to_vec())
            .send()
            .await
            .context("PUT")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("PUT {key} -> {status}: {body}"));
        }
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Bytes> {
        let url = self.host_for(&self.endpoint, key);
        let host = self.host_header_value(&self.endpoint);
        let canonical_uri = format!("/{}/{}", self.bucket, key);

        let mut headers = self.sign_request("GET", &canonical_uri, &host, "", &sha256_hex(b""), None);
        headers.insert(reqwest::header::HOST, HeaderValue::from_str(&host).unwrap());

        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .context("GET")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("GET {key} -> {status}: {body}"));
        }
        let bytes = resp.bytes().await.context("read body")?;
        Ok(bytes)
    }

    async fn list(&self, prefix: &str, limit: usize) -> Result<Vec<ObjectInfo>> {
        let query = format!("list-type=2&prefix={}&max-keys={}", urlencoding(prefix), limit);
        let url = format!(
            "{}/{}?{}",
            self.endpoint.trim_end_matches('/'),
            self.bucket,
            query
        );
        let host = self.host_header_value(&self.endpoint);
        let canonical_uri = format!("/{}", self.bucket);

        let mut headers = self.sign_request("GET", &canonical_uri, &host, &query, &sha256_hex(b""), None);
        headers.insert(reqwest::header::HOST, HeaderValue::from_str(&host).unwrap());

        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .context("LIST")?;
        if !resp.status().is_success() {
            return Err(anyhow!("LIST -> {}", resp.status()));
        }
        let body = resp.text().await.context("read list body")?;

        let mut out = Vec::new();
        for chunk in body.split("</Contents>") {
            if !chunk.contains("<Key>") { continue; }
            let key = extract_tag(chunk, "Key").unwrap_or_default();
            let size = extract_tag(chunk, "Size")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            let etag = extract_tag(chunk, "ETag");
            out.push(ObjectInfo {
                key,
                size,
                content_type: None,
                etag,
                last_modified: None,
            });
            if out.len() >= limit { break; }
        }
        Ok(out)
    }

    async fn presigned_get(&self, key: &str, ttl_secs: u64) -> Result<String> {
        let now = Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();
        let expires = ttl_secs.to_string();
        let host = self.host_header_value(&self.public_endpoint);

        let credential_scope =
            format!("{date_stamp}/{}/{}/aws4_request", self.region, "s3");
        let credential = format!("{}/{}", self.access_key, credential_scope);

        let mut qparams: Vec<(String, String)> = vec![
            ("X-Amz-Algorithm".into(), "AWS4-HMAC-SHA256".into()),
            ("X-Amz-Credential".into(), credential.clone()),
            ("X-Amz-Date".into(), amz_date.clone()),
            ("X-Amz-Expires".into(), expires.clone()),
            ("X-Amz-SignedHeaders".into(), "host".into()),
        ];
        qparams.sort_by(|a, b| a.0.cmp(&b.0));

        let canonical_query = qparams
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding(k), urlencoding(v)))
            .collect::<Vec<_>>()
            .join("&");

        let canonical_path = format!("/{}/{}", self.bucket, key);
        let canonical_request = format!(
            "GET\n{canonical_path}\n{canonical_query}\nhost:{host}\n\nhost\nUNSIGNED-PAYLOAD"
        );
        let cr_hash = sha256_hex(canonical_request.as_bytes());

        let string_to_sign =
            format!("AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{cr_hash}");

        let signing_key = derive_key(&self.secret_key, &date_stamp, &self.region, "s3");
        let signature = {
            let mut mac = HmacSha256::new_from_slice(&signing_key).unwrap();
            mac.update(string_to_sign.as_bytes());
            hex::encode(mac.finalize().into_bytes())
        };

        let url = format!(
            "{}{}?{}&X-Amz-Signature={}",
            self.public_endpoint.trim_end_matches('/'),
            canonical_path,
            canonical_query,
            signature
        );
        Ok(url)
    }
}

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim().to_string())
}

fn urlencoding(s: &str) -> String {
    // Encoding estilo AWS: espacios como %20, no +
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}