use anyhow::{anyhow, Result};
use chrono::Local;
use clap::{CommandFactory, Parser, Subcommand};
use dialoguer::Input;
use futures_util::StreamExt;
use keyring::Entry;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use serde_json;

const CONVERSATION_API_URL: &str = "https://api.1min.ai/api/conversations";
const STREAMING_FEATURES_API_URL: &str = "https://api.1min.ai/api/features?isStreaming=true";
const IMAGE_GENERATION_API_URL: &str = "https://api.1min.ai/api/features";
const DEFAULT_MODEL: &str = "o3-mini";
const DEFAULT_IMAGE_MODEL: &str = "dall-e-3";
const DEFAULT_IMAGE_SIZE: &str = "1024x1024";
const DEFAULT_IMAGE_QUALITY: &str = "standard";
const DEFAULT_IMAGE_STYLE: &str = "vivid";
const MAX_WORDS: u32 = 500;
const SERVICE_NAME: &str = "ai-cli";
const USERNAME: &str = "user";
const DEFAULT_IMAGE_FILENAME: &str = "1minAI_output.png";

#[derive(Parser)]
#[command(author, version, about = "CLI tool for interacting with 1min.ai API")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// The prompt to send to the AI
    prompt: Option<String>,

    /// Enable interactive mode
    #[arg(short, long)]
    interactive: bool,

    /// Enable voice output of AI responses
    #[arg(short, long)]
    voice_output: bool,

    /// Do not print AI responses (only works with voice output)
    #[arg(short, long)]
    quiet: bool,

    /// The AI model to use
    #[arg(short, long, default_value = DEFAULT_MODEL)]
    model: String,

    /// Maximum number of words for web search
    #[arg(short, long, default_value_t = MAX_WORDS)]
    words: u32,
    
    /// Enable image generation mode (incompatible with interactive and voice modes)
    #[arg(short = 'g', long)]
    image_generation: bool,
    
    /// Image size (1024x1024, 1024x1792, 1792x1024)
    #[arg(short, long, default_value = DEFAULT_IMAGE_SIZE)]
    size: String,
    
    /// Image quality (standard, hd)
    #[arg(long, default_value = DEFAULT_IMAGE_QUALITY)]
    quality: String,
    
    /// Image style (vivid, natural)
    #[arg(long, default_value = DEFAULT_IMAGE_STYLE)]
    style: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Configure API key
    Config,
}

#[derive(Serialize)]
struct ConversationRequest {
    #[serde(rename = "type")]
    request_type: String,
    title: String,
}

#[derive(Deserialize)]
struct ConversationResponse {
    conversation: Conversation,
}

#[derive(Deserialize)]
struct Conversation {
    uuid: String,
}

#[derive(Serialize)]
struct ChatRequest {
    #[serde(rename = "type")]
    request_type: String,
    #[serde(rename = "conversationId")]
    conversation_id: String,
    model: String,
    #[serde(rename = "promptObject")]
    prompt_object: PromptObject,
}

#[derive(Serialize)]
struct PromptObject {
    prompt: String,
    #[serde(rename = "isMixed")]
    is_mixed: bool,
    #[serde(rename = "webSearch")]
    web_search: bool,
    #[serde(rename = "numOfSite")]
    num_of_site: u32,
    #[serde(rename = "maxWord")]
    max_word: u32,
}

#[derive(Serialize)]
struct ImageGenerationRequest {
    #[serde(rename = "type")]
    request_type: String,
    model: String,
    #[serde(rename = "promptObject")]
    prompt_object: ImagePromptObject,
}

#[derive(Serialize)]
struct ImagePromptObject {
    #[serde(rename = "prompt")]
    prompt: String,
    #[serde(rename = "n")]
    n: u32,
    #[serde(rename = "size")]
    size: String,
    #[serde(rename = "quality")]
    quality: String,
    #[serde(rename = "style")]
    style: String,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct ImageGenerationResponse {
    aiRecord: AIRecord,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
struct AIRecord {
    #[serde(default)]
    temporaryUrl: String,
    status: String,
    #[serde(default)]
    aiRecordDetail: Option<AIRecordDetail>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
struct AIRecordDetail {
    #[serde(default)]
    resultObject: Option<Vec<String>>,
}

async fn get_api_key() -> Result<String> {
    let keyring = Entry::new(SERVICE_NAME, USERNAME)?;
    
    match keyring.get_password() {
        Ok(key) => Ok(key),
        Err(_) => {
            let api_key: String = Input::<String>::new()
                .with_prompt("API key not found. Please enter your 1min.ai API key")
                .allow_empty(false)
                .interact()?;
            
            keyring.set_password(&api_key)?;
            Ok(api_key)
        }
    }
}

async fn set_api_key() -> Result<()> {
    let api_key: String = Input::<String>::new()
        .with_prompt("Please enter your 1min.ai API key")
        .allow_empty(false)
        .interact()?;
    
    let keyring = Entry::new(SERVICE_NAME, USERNAME)?;
    keyring.set_password(&api_key)?;
    println!("API key saved successfully!");
    Ok(())
}

async fn initialize_conversation(client: &Client, api_key: &str, prompt: &str) -> Result<String> {
    let now = Local::now().format("%Y/%m/%d at %I:%M:%S %p").to_string();
    
    let request = ConversationRequest {
        request_type: "CHAT_WITH_AI".to_string(),
        title: format!("API - {}", now),
    };

    let response = client
        .post(CONVERSATION_API_URL)
        .header("API-KEY", api_key)
        .header(header::CONTENT_TYPE, "application/json")
        .json(&request)
        .send()
        .await?;

    if response.status().is_success() {
        let conversation: ConversationResponse = response.json().await?;
        Ok(conversation.conversation.uuid)
    } else {
        let status = response.status();
        let text = response.text().await?;
        
        if status.as_u16() == 401 {
            let new_api_key: String = Input::<String>::new()
                .with_prompt("Invalid API key. Please enter a new one")
                .allow_empty(false)
                .interact()?;
            
            let keyring = Entry::new(SERVICE_NAME, USERNAME)?;
            keyring.set_password(&new_api_key)?;
            
            Box::pin(initialize_conversation(client, &new_api_key, prompt)).await
        } else {
            Err(anyhow!("Error communicating with conversation API: {} - {}", status, text))
        }
    }
}

async fn chat_with_ai(
    client: &Client, 
    api_key: &str,
    conversation_uuid: &str, 
    prompt: &str,
    model: &str,
    max_words: u32,
    quiet: bool,
    voice_output: bool,
) -> Result<()> {
    let request = ChatRequest {
        request_type: "CHAT_WITH_AI".to_string(),
        conversation_id: conversation_uuid.to_string(),
        model: model.to_string(),
        prompt_object: PromptObject {
            prompt: prompt.to_string(),
            is_mixed: false,
            web_search: false,
            num_of_site: 0,
            max_word: max_words,
        },
    };

    let response = client
        .post(STREAMING_FEATURES_API_URL)
        .header("API-KEY", api_key)
        .header(header::CONTENT_TYPE, "application/json")
        .json(&request)
        .send()
        .await?;

    if response.status().is_success() {
        if !quiet {
            print!("AI({}): ", model);
            io::stdout().flush()?;
        }

        let mut stream = response.bytes_stream();
        let mut full_response = String::with_capacity(1024);

        while let Some(item) = stream.next().await {
            let chunk = item?;
            let text_chunk = String::from_utf8_lossy(&chunk);
            
            if !quiet {
                print!("{}", text_chunk);
                io::stdout().flush()?;
            }
            
            full_response.push_str(&text_chunk);
        }

        if !quiet {
            println!();
        }

        if voice_output {
            speak_response(&full_response)?;
        }

        Ok(())
    } else {
        let status = response.status();
        let text = response.text().await?;
        
        if status.as_u16() == 401 {
            let new_api_key: String = Input::<String>::new()
                .with_prompt("Invalid API key. Please enter a new one")
                .allow_empty(false)
                .interact()?;
            
            let keyring = Entry::new(SERVICE_NAME, USERNAME)?;
            keyring.set_password(&new_api_key)?;
            
            Box::pin(chat_with_ai(
                client, 
                &new_api_key, 
                conversation_uuid, 
                prompt, 
                model, 
                max_words, 
                quiet, 
                voice_output
            )).await
        } else {
            Err(anyhow!("Error communicating with features API: {} - {}", status, text))
        }
    }
}

fn speak_response(text: &str) -> Result<()> {
    Command::new("say")
        .arg(text)
        .spawn()?
        .wait()?;
    Ok(())
}

async fn generate_image(client: &Client, api_key: &str, prompt: &str, model: &str, size: &str, quality: &str, style: &str) -> Result<()> {
    println!("Generating image with {} model for prompt \"{}\"...", model, prompt);
    
    let request = ImageGenerationRequest {
        request_type: "IMAGE_GENERATOR".to_string(),
        model: model.to_string(),
        prompt_object: ImagePromptObject {
            prompt: prompt.to_string(),
            n: 1,
            size: size.to_string(),
            quality: quality.to_string(),
            style: style.to_string(),
        },
    };

    let response = client
        .post(IMAGE_GENERATION_API_URL)
        .header("API-KEY", api_key)
        .header(header::CONTENT_TYPE, "application/json")
        .json(&request)
        .send()
        .await?;

    if response.status().is_success() {
        let response_text = response.text().await?;
        let image_response: ImageGenerationResponse = serde_json::from_str(&response_text)?;
        
        if image_response.aiRecord.status != "SUCCESS" {
            return Err(anyhow!("Image generation failed with status: {}", image_response.aiRecord.status));
        }
        
        println!("Image generated successfully. Downloading...");
        
        if image_response.aiRecord.temporaryUrl.is_empty() {
            return Err(anyhow!("No image URL found in response"));
        }
        
        let url = &image_response.aiRecord.temporaryUrl;
        let filename = url.split('?').next()
            .and_then(|path| path.split('/').last())
            .unwrap_or(DEFAULT_IMAGE_FILENAME);
        
        let image_bytes = client
            .get(url)
            .send()
            .await?
            .bytes()
            .await?;
            
        let path = Path::new(filename);
        let mut file = File::create(path)?;
        file.write_all(&image_bytes)?;
        
        println!("Image saved to {}", filename);
        Ok(())
    } else {
        let status = response.status();
        let text = response.text().await?;
        
        if status.as_u16() == 401 {
            let new_api_key: String = Input::<String>::new()
                .with_prompt("Invalid API key. Please enter a new one")
                .allow_empty(false)
                .interact()?;
            
            let keyring = Entry::new(SERVICE_NAME, USERNAME)?;
            keyring.set_password(&new_api_key)?;
            
            Box::pin(generate_image(client, &new_api_key, prompt, model, size, quality, style)).await
        } else {
            let error_message = match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(json) => {
                    if let Some(message) = json.get("message").and_then(|m| m.as_str()) {
                        if let Some(space_pos) = message.find(' ') {
                            if message[..space_pos].parse::<u32>().is_ok() {
                                message[space_pos+1..].to_string()
                            } else {
                                message.to_string()
                            }
                        } else {
                            message.to_string()
                        }
                    } else {
                        text
                    }
                },
                Err(_) => text,
            };
            
            Err(anyhow!("{} - {}", status, error_message))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = Client::new();

    match &cli.command {
        Some(Commands::Config) => {
            set_api_key().await?;
            return Ok(());
        }
        None => {}
    }

    let api_key = get_api_key().await?;

    let mut errors = Vec::with_capacity(3);
    
    if cli.quiet && !cli.voice_output {
        errors.push("Quiet mode requires voice output to be enabled.");
    }
    
    if cli.image_generation && cli.interactive {
        errors.push("Image generation is not compatible with interactive mode.");
    }
    
    if cli.image_generation && cli.voice_output {
        errors.push("Image generation is not compatible with voice output mode.");
    }
    
    if !errors.is_empty() {
        return Err(anyhow!("{}", errors.join("\nError: ")));
    }

    if cli.image_generation {
        match &cli.prompt {
            Some(prompt) => {
                let model = if cli.model == DEFAULT_MODEL {
                    DEFAULT_IMAGE_MODEL
                } else {
                    &cli.model
                };
                
                generate_image(&client, &api_key, prompt, model, &cli.size, &cli.quality, &cli.style).await?;
                return Ok(());
            }
            None => {
                return Err(anyhow!("Error: No prompt provided for image generation."));
            }
        }
    }

    let prompt = match &cli.prompt {
        Some(p) => p.as_str(),
        None => "",
    };
    
    let conversation_uuid = initialize_conversation(&client, &api_key, prompt).await?;

    if cli.interactive {
        println!("Starting interactive mode. Type 'exit' to quit.");
        
        let mut prompt = match &cli.prompt {
            Some(p) => {
                println!("You: {}", p);
                p.clone()
            }
            None => {
                let input: String = Input::new().with_prompt("You").interact_text()?;
                input
            }
        };

        while !prompt.is_empty() && prompt.to_lowercase() != "exit" {
            chat_with_ai(
                &client,
                &api_key,
                &conversation_uuid,
                &prompt,
                &cli.model,
                cli.words,
                cli.quiet,
                cli.voice_output,
            ).await?;

            prompt = Input::new().with_prompt("You").interact_text()?;
        }
    } else {
        match &cli.prompt {
            Some(prompt) => {
                chat_with_ai(
                    &client,
                    &api_key,
                    &conversation_uuid,
                    prompt,
                    &cli.model,
                    cli.words,
                    cli.quiet,
                    cli.voice_output,
                ).await?;
            }
            None => {
                Cli::command().print_help()?;
                return Ok(());
            }
        }
    }

    Ok(())
}
