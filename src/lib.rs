use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use protobuf::descriptor::{FileDescriptorProto, FileDescriptorSet};
use protobuf::error::ProtobufError;
use protobuf::error::ProtobufResult;
use protobuf::error::WireError;
use protobuf::Message;
use protobuf::{CodedInputStream, CodedOutputStream};
use serde::de::Deserialize;
use serde_json::Value;
use serde_protobuf::de::Deserializer;
use serde_protobuf::descriptor::Descriptors;
use std::fs;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::{Read, Write};
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

        Ok(PBZReader {
            gz: BufReader::new(gz),
            descriptors: Descriptors::new(),
            next_descriptor_name: String::from(""),
        })
    }

    fn read_raw_byte(&mut self) -> io::Result<u8> {
        let mut buf = [0; 1];
        self.gz.read_exact(&mut buf)?;
        Ok(buf[0])
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
            mlen |= ((b & 0x7f) as u64) << (i * 7);
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
        Ok((mtype, buf))
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
        T::parse_from_bytes(&buf)
    }

    pub fn next_value(&mut self) -> ProtobufResult<Value> {
        let buf = self.next_message_buffer()?;
        let input = CodedInputStream::from_bytes(&buf);
        let deserializer =
            Deserializer::for_named_message(&self.descriptors, &self.next_descriptor_name, input);
        let value = Value::deserialize(&mut deserializer.unwrap());
        if let Ok(val) = value {
            return Ok(val);
        }
        Err(ProtobufError::WireError(WireError::Other))
    }
}

pub struct PBZWriter {
    gz: GzEncoder<BufWriter<File>>,
    descriptors: Descriptors,
    last_descriptor_name: String,
}

impl PBZWriter {
    pub fn new(filename: &str) -> io::Result<PBZWriter> {
        let file = File::create(filename).unwrap();
        let writer = BufWriter::new(file);
        let mut gz = GzEncoder::new(writer, Compression::default());

        gz.write_all(MAGIC)?;
        Ok(PBZWriter {
            gz: gz,
            descriptors: Descriptors::new(),
            last_descriptor_name: String::from(""),
        })
    }

    pub fn write_descriptor_from_file(&mut self, filename: &str) -> io::Result<()> {
        let buf = fs::read(filename)?;
        let fds = FileDescriptorSet::parse_from_bytes(&buf)?;
        self.descriptors.add_file_set_proto(&fds);
        println!("{:?}", self.descriptors);
        let mut cos = CodedOutputStream::new(&mut self.gz);
        cos.write_all(&[T_FILE_DESCRIPTOR])?;
        cos.write_bytes_no_tag(&buf)?;
        cos.flush()?;
        Ok(())
    }

    pub fn write_file_descriptor_proto(&mut self, fdp: &FileDescriptorProto) -> io::Result<()> {
        let mut fds = FileDescriptorSet::new();
        let mut cont: protobuf::RepeatedField<FileDescriptorProto> = protobuf::RepeatedField::new();
        cont.push(fdp.clone());
        fds.set_file(cont);
        self.descriptors.add_file_set_proto(&fds);
        let mut cos = CodedOutputStream::new(&mut self.gz);
        cos.write_all(&[T_FILE_DESCRIPTOR])?;
        cos.write_message_no_tag(&fds)?;
        cos.flush()?;
        Ok(())
    }

    pub fn write<T: protobuf::Message>(&mut self, msg: &T) -> ProtobufResult<()> {
        let descriptor_name = msg.descriptor().full_name().to_string();
        let mut cos = CodedOutputStream::new(&mut self.gz);
        if descriptor_name != self.last_descriptor_name {
            // Make sure that the loaded descriptor contains the message type
            let msg_descriptor = self
                .descriptors
                .message_by_name(&format!(".{}", descriptor_name));
            assert!(msg_descriptor.is_some());

            cos.write_all(&[T_DESCRIPTOR_NAME])?;
            cos.write_string_no_tag(&descriptor_name)?;
            self.last_descriptor_name = descriptor_name;
        }
        cos.write_all(&[T_MESSAGE])?;
        cos.write_message_no_tag(msg)?;
        cos.flush()?;
        Ok(())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.gz.flush()?;
        Ok(())
    }
}
