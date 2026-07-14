use alloc::string::String;

use url::{Position, Url};

use crate::ToQRText;

fn uppercase_percent_escapes(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut normalized = String::with_capacity(text.len());
    let mut copied = 0;
    let mut index = 0;

    // Only complete percent-encoded octets can be changed without changing URL data.
    while index + 2 < bytes.len() {
        if bytes[index] == b'%'
            && bytes[index + 1].is_ascii_hexdigit()
            && bytes[index + 2].is_ascii_hexdigit()
        {
            normalized.push_str(&text[copied..index]);
            normalized.push('%');
            normalized.push(char::from(bytes[index + 1].to_ascii_uppercase()));
            normalized.push(char::from(bytes[index + 2].to_ascii_uppercase()));

            index += 3;
            copied = index;
        } else {
            index += 1;
        }
    }

    normalized.push_str(&text[copied..]);
    normalized
}

impl ToQRText for Url {
    fn to_qr_text(&self) -> String {
        let serialized = self.as_str();
        let scheme_end = self[..Position::AfterScheme].len();
        let path_start = self[..Position::BeforePath].len();
        let path_end = self[..Position::AfterPath].len();
        let omit_root_path = matches!(self.scheme(), "http" | "https") && self.path() == "/";
        let mut normalized = String::with_capacity(serialized.len());

        normalized.push_str(&serialized[..scheme_end].to_ascii_uppercase());

        // URL hosts are case-insensitive, but user information and path data may not be.
        if self.has_host() {
            let host_start = self[..Position::BeforeHost].len();
            let host_end = self[..Position::AfterHost].len();

            normalized.push_str(&serialized[scheme_end..host_start]);
            normalized.push_str(&serialized[host_start..host_end].to_ascii_uppercase());
            normalized.push_str(&serialized[host_end..path_start]);
        } else {
            normalized.push_str(&serialized[scheme_end..path_start]);
        }

        if !omit_root_path {
            normalized.push_str(&serialized[path_start..path_end]);
        }

        normalized.push_str(&serialized[path_end..]);
        uppercase_percent_escapes(&normalized)
    }
}
