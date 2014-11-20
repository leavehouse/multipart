use hyper::client::{Request, Response};
use hyper::header::common::ContentType;
use hyper::net::{Fresh, Streaming};
use hyper::{HttpResult, HttpIoError};

use mime::{Mime, TopLevel, SubLevel, Attr, Value};

use mime_guess::guess_mime_type;

use std::io::IoResult;
use std::io::fs::File;
use std::io;

use super::{MultipartField, MultipartFile};

const BOUNDARY_LEN: uint = 8;

pub struct Multipart<'a> {
    fields: Vec<(String, MultipartField<'a>)>,
    boundary: String,
}

/// Shorthand for a writable request (`Request<Streaming>`)
type ReqWrite = Request<Streaming>;

impl<'a> Multipart<'a> {

    pub fn new() -> Multipart<'a> {
        Multipart {
            fields: Vec::new(),
            boundary: random_alphanumeric(BOUNDARY_LEN),
        } 
    }

    pub fn add_text(&mut self, name: &str, val: &str) {
        self.fields.push((name.into_string(), MultipartField::Text(val.into_string())));    
    }
    
    /// Add the file to the multipart request, guessing its `Content-Type` from its extension
    pub fn add_file(&mut self, name: &str, file: &'a mut File) {
        let filename = file.path().filename_str().map(|s| s.into_string());
        let content_type = guess_mime_type(file.path());

        self.fields.push((name.into_string(), 
            MultipartField::File(MultipartFile::from_file(filename, file, content_type))));
    }

    /// Apply the appropriate headers to the `Request<Fresh>` and send the data.
    pub fn send(self, mut req: Request<Fresh>) -> HttpResult<Response> {
        use hyper::method;
        assert!(req.method() == method::Post, "Multipart request must use POST method!");

        self.apply_headers(&mut req);

        debug!("Fields: {}; Boundary: {}", self.fields[], self.boundary[]);

        debug!("{}", req.headers());

        let mut req = try!(req.start());
        try!(io_to_http(self.write_request(&mut req)));
        req.send()
    }
    
    fn apply_headers(&self, req: &mut Request<Fresh>){
        let headers = req.headers_mut();

        headers.set(ContentType(multipart_mime(self.boundary[])))         
    }

    fn write_request(self, req: &mut ReqWrite) -> IoResult<()> {
        let Multipart{ fields, boundary } = self;

        try!(write_boundary(req, boundary[]));

        for (name, field) in fields.into_iter() {
            try!(write!(req, "Content-Disposition: form-data; name=\"{}\"\r\n\r\n", name));

            try!(match field {
                    MultipartField::Text(text) => write_line(req, &*text),
                    MultipartField::File(file) => write_file(req, file),
                });
            
            try!(write_boundary(req, boundary[]));     
        }

        Ok(())
    }

}

fn write_boundary(req: &mut ReqWrite, boundary: &str) -> IoResult<()> {
    write!(req, "--{}\r\n", boundary)
}

fn write_file(req: &mut ReqWrite, mut file: MultipartFile) -> IoResult<()> {
    try!(file.filename.map(|filename| write!(req, "; filename=\"{}\"\r\n", filename)).unwrap_or(Ok(())));
    try!(write!(req, "Content-Type: {}\r\n\r\n", file.content_type));
    io::util::copy(&mut file.reader, req)         
}

/// Specialized write_line that writes CRLF after a line as per W3C specs
fn write_line(req: &mut ReqWrite, s: &str) -> IoResult<()> {
    req.write_str(s).and_then(|_| req.write(b"\r\n"))        
}

/// Generate a random alphanumeric sequence of length `len`
fn random_alphanumeric(len: uint) -> String {
    use std::rand::{task_rng, Rng};
    use std::char::to_lowercase;

    task_rng().gen_ascii_chars().map(to_lowercase).take(len).collect()    
}

fn io_to_http<T>(res: IoResult<T>) -> HttpResult<T> {
    res.map_err(|e| HttpIoError(e))
}

fn multipart_mime(bound: &str) -> Mime {
    Mime(
        TopLevel::Multipart, SubLevel::Ext("form-data".into_string()),
        vec![(Attr::Ext("boundary".into_string()), Value::Ext(bound.into_string()))]
    )         
}


