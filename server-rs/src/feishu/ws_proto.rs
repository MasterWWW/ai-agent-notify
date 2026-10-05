// Minimal protobuf codec for the Feishu long-connection Frame/Header messages.
// Schema (from @larksuiteoapi/node-sdk):
//   message Header { required string key = 1; required string value = 2; }
//   message Frame  { required uint64 SeqID = 1; required uint64 LogID = 2;
//                    required int32 service = 3; required int32 method = 4;
//                    repeated Header headers = 5; optional string payloadEncoding = 6;
//                    optional string payloadType = 7; optional bytes payload = 8;
//                    optional string LogIDNew = 9; }

#[derive(Debug, Clone, Default)]
pub struct Header {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Default)]
pub struct Frame {
    pub seq_id: u64,
    pub log_id: u64,
    pub service: i32,
    pub method: i32, // 0 = control, 1 = data
    pub headers: Vec<Header>,
    pub payload_encoding: Option<String>,
    pub payload_type: Option<String>,
    pub payload: Option<Vec<u8>>,
    pub log_id_new: Option<String>,
}

impl Frame {
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers.iter().find(|h| h.key == key).map(|h| h.value.as_str())
    }

    pub fn header_map(&self) -> std::collections::HashMap<String, String> {
        self.headers
            .iter()
            .map(|h| (h.key.clone(), h.value.clone()))
            .collect()
    }
}

fn encode_varint(mut v: u64, out: &mut Vec<u8>) {
    loop {
        let mut b = (v & 0x7f) as u8;
        v >>= 7;
        if v != 0 {
            b |= 0x80;
        }
        out.push(b);
        if v == 0 {
            break;
        }
    }
}

fn encode_tag(field: u32, wire: u32, out: &mut Vec<u8>) {
    encode_varint(((field as u64) << 3) | (wire as u64), out);
}

fn encode_len_field(field: u32, bytes: &[u8], out: &mut Vec<u8>) {
    encode_tag(field, 2, out);
    encode_varint(bytes.len() as u64, out);
    out.extend_from_slice(bytes);
}

fn encode_str_field(field: u32, s: &str, out: &mut Vec<u8>) {
    encode_len_field(field, s.as_bytes(), out);
}

impl Frame {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        if self.seq_id != 0 {
            encode_tag(1, 0, &mut out);
            encode_varint(self.seq_id, &mut out);
        }
        if self.log_id != 0 {
            encode_tag(2, 0, &mut out);
            encode_varint(self.log_id, &mut out);
        }
        if self.service != 0 {
            encode_tag(3, 0, &mut out);
            encode_varint(self.service as u64, &mut out);
        }
        if self.method != 0 {
            encode_tag(4, 0, &mut out);
            encode_varint(self.method as u64, &mut out);
        }
        for h in &self.headers {
            let mut buf = Vec::new();
            encode_str_field(1, &h.key, &mut buf);
            encode_str_field(2, &h.value, &mut buf);
            encode_len_field(5, &buf, &mut out);
        }
        if let Some(pe) = &self.payload_encoding {
            encode_str_field(6, pe, &mut out);
        }
        if let Some(pt) = &self.payload_type {
            encode_str_field(7, pt, &mut out);
        }
        if let Some(p) = &self.payload {
            encode_len_field(8, p, &mut out);
        }
        if let Some(ln) = &self.log_id_new {
            encode_str_field(9, ln, &mut out);
        }
        out
    }
}

fn read_varint(bytes: &[u8], pos: &mut usize) -> Result<u64, String> {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    loop {
        if *pos >= bytes.len() {
            return Err("varint out of bounds".to_string());
        }
        let b = bytes[*pos];
        *pos += 1;
        if shift >= 64 {
            return Err("varint too long".to_string());
        }
        result |= ((b & 0x7f) as u64) << shift;
        if b & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    Ok(result)
}

fn read_bytes<'a>(bytes: &'a [u8], pos: &mut usize) -> Result<&'a [u8], String> {
    let len = read_varint(bytes, pos)? as usize;
    if *pos + len > bytes.len() {
        return Err("bytes out of bounds".to_string());
    }
    let out = &bytes[*pos..*pos + len];
    *pos += len;
    Ok(out)
}

pub fn decode_frame(bytes: &[u8]) -> Result<Frame, String> {
    let mut frame = Frame::default();
    let mut pos = 0usize;
    while pos < bytes.len() {
        let tag = read_varint(bytes, &mut pos)?;
        let field = (tag >> 3) as u32;
        let wire = (tag & 7) as u32;
        match (field, wire) {
            (1, 0) => frame.seq_id = read_varint(bytes, &mut pos)?,
            (2, 0) => frame.log_id = read_varint(bytes, &mut pos)?,
            (3, 0) => frame.service = read_varint(bytes, &mut pos)? as i32,
            (4, 0) => frame.method = read_varint(bytes, &mut pos)? as i32,
            (5, 2) => {
                let hdr_bytes = read_bytes(bytes, &mut pos)?;
                frame.headers.push(decode_header(hdr_bytes)?);
            }
            (6, 2) => {
                frame.payload_encoding =
                    Some(String::from_utf8_lossy(read_bytes(bytes, &mut pos)?).into_owned());
            }
            (7, 2) => {
                frame.payload_type =
                    Some(String::from_utf8_lossy(read_bytes(bytes, &mut pos)?).into_owned());
            }
            (8, 2) => {
                frame.payload = Some(read_bytes(bytes, &mut pos)?.to_vec());
            }
            (9, 2) => {
                frame.log_id_new =
                    Some(String::from_utf8_lossy(read_bytes(bytes, &mut pos)?).into_owned());
            }
            (_, 0) => {
                read_varint(bytes, &mut pos)?;
            }
            (_, 1) => {
                if pos + 8 > bytes.len() {
                    return Err("fixed64 out of bounds".to_string());
                }
                pos += 8;
            }
            (_, 2) => {
                let len = read_varint(bytes, &mut pos)? as usize;
                if pos + len > bytes.len() {
                    return Err("length-delimited out of bounds".to_string());
                }
                pos += len;
            }
            (_, 5) => {
                if pos + 4 > bytes.len() {
                    return Err("fixed32 out of bounds".to_string());
                }
                pos += 4;
            }
            _ => return Err(format!("unsupported wire type {wire}")),
        }
    }
    Ok(frame)
}

fn decode_header(bytes: &[u8]) -> Result<Header, String> {
    let mut h = Header::default();
    let mut pos = 0usize;
    while pos < bytes.len() {
        let tag = read_varint(bytes, &mut pos)?;
        let field = (tag >> 3) as u32;
        let wire = (tag & 7) as u32;
        if (field, wire) == (1, 2) {
            h.key = String::from_utf8_lossy(read_bytes(bytes, &mut pos)?).into_owned();
        } else if (field, wire) == (2, 2) {
            h.value = String::from_utf8_lossy(read_bytes(bytes, &mut pos)?).into_owned();
        } else if wire == 0 {
            read_varint(bytes, &mut pos)?;
        } else if wire == 2 {
            let len = read_varint(bytes, &mut pos)? as usize;
            if pos + len > bytes.len() {
                return Err("header len out of bounds".to_string());
            }
            pos += len;
        } else if wire == 1 {
            pos += 8;
        } else if wire == 5 {
            pos += 4;
        }
    }
    Ok(h)
}
