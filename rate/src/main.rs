use reqwest::blocking::Client;
use reqwest::header::USER_AGENT;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::time::Duration;
use chrono::Local;
use scraper::{Html, Selector};
use toml;

// 结构体表示记录的价格
#[derive(Deserialize)]
struct PriceRecord {
    price: String, // 价格字段
}

// 结构体表示响应数据
#[derive(Deserialize)]
struct ExchangeRateResponse {
    records: Vec<PriceRecord>, // 响应中的价格记录列表
}

#[derive(Deserialize)]
struct Links {
    boc: String,
    ccpr: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    // 默认链接
    let default_boc = "https://www.boc.cn/sourcedb/whpj/";
    let default_ccpr = "https://www.chinamoney.com.cn/r/cms/www/chinamoney/data/fx/ccpr.json";

    // 读取.rate.toml文件，如果存在
    let (boc_url, ccpr_url) = if Path::new(".rate.toml").exists() {
        let config_content = fs::read_to_string(".rate.toml")?;
        let links: Links = toml::from_str(&config_content)?;
        (links.boc, links.ccpr)
    } else {
        (default_boc.to_string(), default_ccpr.to_string())
    };

    // 尝试获取主用汇率，如果失败则启用备用方案
    match fetch_boc_exchange_rate(&boc_url) {
        Ok(rate) => {
            let now = Local::now();
            let formatted_time = now.format("%Y/%m/%d %H:%M").to_string();
            print!("{:.4} # BOC ({})", rate, formatted_time);
        },
        Err(_) => {
            match fetch_ccpr_exchange_rate(&ccpr_url) {
                Ok(rate) => {
                    let now = Local::now();
                    let formatted_time = now.format("%Y/%m/%d %H:%M").to_string();
                    print!("{:.4} # CCPR ({})", rate, formatted_time);
                },
                Err(_) => {
                    print!("两种汇率来源全部失效, 请检查链接!");
                }
            }
        }
    }

    Ok(())
}

fn fetch_ccpr_exchange_rate(url: &str) -> Result<f64, Box<dyn Error>> {
    // 创建一个HTTP客户端
    let client = Client::builder()
        .timeout(Duration::new(10, 0))  // 设置请求超时为10秒
        .danger_accept_invalid_certs(true)  // 接受无效的证书（仅用于测试环境）
        .build()?;
    
    // 发送GET请求
    let response = client.get(url)
        .header(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3")  // 设置User-Agent头部
        .send()?;
    
    // 处理响应
    if response.status().is_success() {
        // 解析JSON响应体
        let response_data: ExchangeRateResponse = response.json()?;
        // 如果有记录，返回第一条记录的价格
        if let Some(record) = response_data.records.get(0) {
            let rate: f64 = record.price.parse()?;
            return Ok(rate);
        } else {
            return Err("没有找到记录.".into());
        }
    } else {
        return Err(format!("下载文件失败. 状态: {}", response.status()).into());
    }
}

fn fetch_boc_exchange_rate(url: &str) -> Result<f64, Box<dyn Error>> {
    let client = Client::builder()
        .timeout(Duration::new(10, 0))  // 设置请求超时为10秒
        .danger_accept_invalid_certs(true)  // 接受无效的证书（仅用于测试环境）
        .build()?;
    
    let response = client.get(url)
        .header(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3")  // 设置User-Agent头部
        .send()?;

    let body = response.text()?;
    
    let document = Html::parse_document(&body);
    let selector = Selector::parse("td").unwrap();
    let mut found_usd = false;
    let mut count = 0;

    for element in document.select(&selector) {
        let text = element.text().collect::<Vec<_>>().concat();
        if found_usd {
            count += 1;
            if count == 5 {
                let rate: f64 = text.parse()?;
                return Ok(rate / 100.0); // 将获取的数字除以100
            }
        }
        if text == "美元" {
            found_usd = true;
        }
    }
    
    Err("从主用来源解析汇率失败.".into())
}
