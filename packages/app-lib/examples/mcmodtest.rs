//! MC 百科搜索解析自测
fn main() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AirCore/1.0")
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .unwrap();
        let url = "https://search.mcmod.cn/s?key=%E8%88%AA%E7%A9%BA%E5%AD%A6";
        match client.get(url).send().await {
            Ok(resp) => {
                println!("status: {}", resp.status());
                println!("final url: {}", resp.url());
                let body = resp.text().await.unwrap_or_default();
                println!("body len: {}", body.len());
                println!("parsed: {:?}", parse_english(&body));
            }
            Err(error) => println!("request error: {error}"),
        }
    });
}

fn parse_english(body: &str) -> Option<String> {
    let mut rest = body;
    while let Some(index) = rest.find("/class/") {
        let tail = &rest[index + "/class/".len()..];
        let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            if let Some(close) = tail.find("</a>") {
                let anchor = &tail[..close];
                if let (Some(open_paren), Some(close_paren)) = (anchor.rfind('('), anchor.rfind(')')) {
                    if close_paren > open_paren + 1 {
                        let candidate = anchor[open_paren + 1..close_paren].trim();
                        if candidate.chars().any(|c| c.is_ascii_alphabetic()) {
                            return Some(candidate.to_string());
                        }
                    }
                }
            }
        }
        rest = tail;
    }
    None
}
