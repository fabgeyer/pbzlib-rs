# Library for serializing protobuf objects - Rust version

This library is used for simplifying the serialization and deserialization of [protocol buffer](https://developers.google.com/protocol-buffers/) objects to/from files.
The main use-case is to save and read a large collection of objects of the same type.
Each file contains a header with the description of the protocol buffer, meaning that no compilation of `.proto` description file is required before reading a `pbz` file.

## Versions in other languages

- [Python version](https://github.com/fabgeyer/pbzlib-py)
- [Go version](https://github.com/fabgeyer/pbzlib-go)
- [Java version](https://github.com/fabgeyer/pbzlib-java)
- [C/C++ version](https://github.com/fabgeyer/pbzlib-c-cpp)
