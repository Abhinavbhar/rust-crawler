use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use std::error::Error;
use std::fs::File;
use std::io::Write;
use url::Url;

#[derive(Debug, Clone)]
struct PageData {
    url: String,
    title: String,
    text_content: String,
    depth: usize,
    parent_url: String,
}

struct Crawler {
    client: Client,
    visited: HashSet<String>,
    max_depth: usize,
    base_domain: String,
}

impl Crawler {
    fn new(max_depth: usize, base_url: &str) -> Result<Self, Box<dyn Error>> {
        let url = Url::parse(base_url)?;
        let base_domain = url.host_str().unwrap_or("").to_string();
        
        Ok(Crawler {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (compatible; RustCrawler/1.0)")
                .timeout(std::time::Duration::from_secs(10))
                .build()?,
            visited: HashSet::new(),
            max_depth,
            base_domain,
        })
    }

    fn normalize_url(&self, url: &str) -> Option<String> {
        if let Ok(parsed) = Url::parse(url) {
            let mut normalized = parsed;
            normalized.set_fragment(None);
            Some(normalized.to_string())
        } else {
            None
        }
    }

    fn is_same_domain(&self, url: &str) -> bool {
        if let Ok(parsed) = Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                return host == self.base_domain;
            }
        }
        false
    }

    fn extract_text(&self, html: &Html) -> String {
        let text_selector = Selector::parse("body").unwrap();
        
        let mut text_parts = Vec::new();
        
        if let Some(body) = html.select(&text_selector).next() {
            for node in body.descendants() {
                if let Some(text) = node.value().as_text() {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        text_parts.push(trimmed.to_string());
                    }
                }
            }
        }
        
        text_parts.join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn extract_links(&self, html: &Html, base_url: &str) -> Vec<String> {
        let link_selector = Selector::parse("a[href]").unwrap();
        let mut links = Vec::new();
        
        if let Ok(base) = Url::parse(base_url) {
            for element in html.select(&link_selector) {
                if let Some(href) = element.value().attr("href") {
                    if let Ok(absolute_url) = base.join(href) {
                        let url_str = absolute_url.to_string();
                        if self.is_same_domain(&url_str) {
                            if let Some(normalized) = self.normalize_url(&url_str) {
                                links.push(normalized);
                            }
                        }
                    }
                }
            }
        }
        
        links
    }

    fn crawl(&mut self, start_url: &str) -> Result<Vec<PageData>, Box<dyn Error>> {
        let mut queue = VecDeque::new();
        let mut results = Vec::new();
        
        if let Some(normalized) = self.normalize_url(start_url) {
            queue.push_back((normalized.clone(), 0, String::from("ROOT")));
        }
        
        while let Some((url, depth, parent)) = queue.pop_front() {
            if depth > self.max_depth {
                continue;
            }
            
            if self.visited.contains(&url) {
                continue;
            }
            
            self.visited.insert(url.clone());
            
            println!("Crawling [depth {}]: {}", depth, url);
            
            match self.client.get(&url).send() {
                Ok(response) => {
                    if !response.status().is_success() {
                        eprintln!("Failed to fetch {}: status {}", url, response.status());
                        continue;
                    }
                    
                    match response.text() {
                        Ok(body) => {
                            let html = Html::parse_document(&body);
                            
                            let title_selector = Selector::parse("title").unwrap();
                            let title = html
                                .select(&title_selector)
                                .next()
                                .map(|t| t.text().collect::<String>())
                                .unwrap_or_else(|| String::from("No Title"));
                            https://github.com/
                            let text = self.extract_text(&html);
                            
                            results.push(PageData {
                                url: url.clone(),
                                title: title.trim().to_string(),
                                text_content: text,
                                depth,
                                parent_url: parent.clone(),
                            });
                            
                            if depth < self.max_depth {
                                let links = self.extract_links(&html, &url);
                                for link in links {
                                    if !self.visited.contains(&link) {
                                        queue.push_back((link, depth + 1, url.clone()));
                                    }
                                }
                            }
                        }
                        Err(e) => eprintln!("Failed to read body for {}: {}", url, e),
                    }
                }
                Err(e) => eprintln!("Failed to fetch {}: {}", url, e),
            }
        }
        
        Ok(results)
    }

    fn save_to_csv(&self, data: &[PageData], filename: &str) -> Result<(), Box<dyn Error>> {
        let mut file = File::create(filename)?;
        
        writeln!(file, "Depth,Parent URL,URL,Title,Text Content (First 500 chars)")?;
        
        for page in data {
            let text_preview = if page.text_content.len() > 500 {
                format!("{}...", &page.text_content[..500])
            } else {
                page.text_content.clone()
            };
            
            let escaped_title = page.title.replace('"', "\"\"");
            let escaped_text = text_preview.replace('"', "\"\"");
            let escaped_parent = page.parent_url.replace('"', "\"\"");
            let escaped_url = page.url.replace('"', "\"\"");
            
            writeln!(
                file,
                "{},\"{}\",\"{}\",\"{}\",\"{}\"",
                page.depth,
                escaped_parent,
                escaped_url,
                escaped_title,
                escaped_text
            )?;
        }
        
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() != 3 {
        eprintln!("Usage: {} <url> <max_depth>", args[0]);
        eprintln!("Example: {} https://example.com 3", args[0]);
        std::process::exit(1);
    }
    
    let start_url = &args[1];
    let max_depth: usize = args[2].parse()
        .expect("Max depth must be a valid number");
    
    println!("Starting crawler...");
    println!("URL: {}", start_url);
    println!("Max Depth: {}", max_depth);
    println!();
    
    let mut crawler = Crawler::new(max_depth, start_url)?;
    let results = crawler.crawl(start_url)?;
    
    println!("\nCrawling complete!");
    println!("Total pages crawled: {}", results.len());
    
    let output_file = "crawl_results.csv";
    crawler.save_to_csv(&results, output_file)?;
    
    println!("Results saved to: {}", output_file);
    
    Ok(())
}