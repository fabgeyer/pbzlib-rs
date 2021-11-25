use flate2::read::GzDecoder;
use protobuf::descriptor::FileDescriptorSet;
use protobuf::error::ProtobufError;
use protobuf::error::ProtobufResult;
use protobuf::error::WireError;
use protobuf::CodedInputStream;
use protobuf::Message;
use serde::de::Deserialize;
use serde_json::Value;
use serde_protobuf::de::Deserializer;
use serde_protobuf::descriptor::Descriptors;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::Read;
use std::string::String;

const MAGIC: &[u8] = &[0x41, 0x42];
const T_FILE_DESCRIPTOR: u8 = 1;
const T_DESCRIPTOR_NAME: u8 = 2;
const T_MESSAGE: u8 = 3;
const T_PROTOBUF_VERSION: u8 = 4;

pub struct PBZReader {
    gz: BufReader<GzDecoder<BufReader<File>>>,
    descriptors: Descriptors,
    next_descriptor_name: String,
}

impl PBZReader {
    pub fn new(filename: &str) -> io::Result<PBZReader> {
        let file = File::open(filename).unwrap();
        let reader = BufReader::new(file);
        let mut gz = GzDecoder::new(reader);

        let mut buffer = [0; 2];
        gz.read_exact(&mut buffer)?;
        if buffer != MAGIC {
            return Err(io::Error::new(io::ErrorKind::Other, "oh no!"));
        }

        return Ok(PBZReader {
            gz: BufReader::new(gz),
            descriptors: Descriptors::new(),
            next_descriptor_name: String::from(""),
        });
    }

    fn read_raw_byte(&mut self) -> io::Result<u8> {
        let mut buf = [0; 1];
        self.gz.read_exact(&mut buf)?;
        return Ok(buf[0]);
    }

    fn read_u64(&mut self) -> ProtobufResult<u64> {
        // Code copied from CodedInputStream
        let mut mlen: u64 = 0;
        let mut i = 0;
        loop {
            if i == 10 {
                return Err(ProtobufError::WireError(WireError::IncorrectVarint));
            }
            let b = self.read_raw_byte()?;
            if i == 9 && (b & 0x7f) > 1 {
                return Err(ProtobufError::WireError(WireError::IncorrectVarint));
            }
            mlen = mlen | (((b & 0x7f) as u64) << (i * 7));
            i += 1;
            if b < 0x80 {
                return Ok(mlen);
            }
        }
    }

    fn read_raw_object(&mut self) -> ProtobufResult<(u8, Vec<u8>)> {
        let mtype = self.read_raw_byte()?;
        let mlen = self.read_u64()?;
        let mut buf = vec![0; mlen as usize];
        self.gz.read_exact(&mut buf)?;
        return Ok((mtype, buf));
    }

    fn next_message_buffer(&mut self) -> ProtobufResult<Vec<u8>> {
        loop {
            let (mtype, buf) = self.read_raw_object()?;

            /*
            match mtype {
                T_FILE_DESCRIPTOR => println!("T_FILE_DESCRIPTOR"),
                T_DESCRIPTOR_NAME => println!("T_DESCRIPTOR_NAME"),
                T_MESSAGE => println!("T_MESSAGE"),
                T_PROTOBUF_VERSION => println!("T_PROTOBUF_VERSION"),
                _ => println!("Unknown mtype {}", mtype),
            }
            */

            if mtype == T_FILE_DESCRIPTOR {
                let fds = FileDescriptorSet::parse_from_bytes(&buf)?;
                self.descriptors.add_file_set_proto(&fds);
                continue;
            } else if mtype == T_MESSAGE {
                return Ok(buf);
            } else if mtype == T_DESCRIPTOR_NAME {
                let name = String::from_utf8(buf).unwrap();
                self.next_descriptor_name = format!(".{}", name);
                continue;
            } else if mtype == T_PROTOBUF_VERSION {
                let _version = String::from_utf8(buf).unwrap();
                continue;
            }
            return Err(ProtobufError::WireError(WireError::Other));
        }
    }

    pub fn next<T: protobuf::Message>(&mut self) -> ProtobufResult<T> {
        let buf = self.next_message_buffer()?;
        return T::parse_from_bytes(&buf);
    }

    pub fn next_value(&mut self) -> ProtobufResult<Value> {
        let buf = self.next_message_buffer()?;
        let input = CodedInputStream::from_bytes(&buf);
        let deserializer =
            Deserializer::for_named_message(&self.descriptors, &self.next_descriptor_name, input);
        let value = Value::deserialize(&mut deserializer.unwrap());
        if value.is_ok() {
            return Ok(value.unwrap());
        }
        return Err(ProtobufError::WireError(WireError::Other));
    }
}
