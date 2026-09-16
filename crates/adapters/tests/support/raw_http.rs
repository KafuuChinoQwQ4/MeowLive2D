use tokio::{io::AsyncReadExt, net::TcpStream};

pub async fn consume_request(stream: &mut TcpStream) {
    let mut headers = Vec::new();
    while !headers.ends_with(b"\r\n\r\n") {
        assert!(headers.len() < 16384, "test request headers exceeded limit");
        headers.push(stream.read_u8().await.unwrap());
    }
    let headers = String::from_utf8(headers).unwrap();
    let length = headers
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    assert!(length <= 16384);
    let mut body = vec![0; length];
    stream.read_exact(&mut body).await.unwrap();
}
