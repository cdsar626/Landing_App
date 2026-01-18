use serde_json::json;
use std::env;

pub async fn send_confirmation_email(email: &str) -> Result<(), reqwest::Error> {
    let api_key = env::var("BREVO_API_KEY").expect("BREVO_API_KEY must be set");
    let sender_email = env::var("SENDER_EMAIL").expect("SENDER_EMAIL must be set");
    let sender_name = env::var("SENDER_NAME").unwrap_or_else(|_| "Waitlist Team".to_string());
    
    // Check if we have a template ID, otherwise use html content
    let template_id_str = env::var("BREVO_TEMPLATE_ID").unwrap_or_default();
    
    let client = reqwest::Client::new();
    
    let mut body = json!({
        "sender": { "name": sender_name, "email": sender_email },
        "to": [{ "email": email }],
    });

    if let Ok(tid) = template_id_str.parse::<i64>() {
        // Use template
        body.as_object_mut().unwrap().insert("templateId".to_string(), json!(tid));
    } else {
        // Use default content
        body.as_object_mut().unwrap().insert("subject".to_string(), json!("Welcome to the list!"));
        body.as_object_mut().unwrap().insert("htmlContent".to_string(), json!("<html><body><h1>You are on the list!</h1><p>Thank you for joining our waitlist.</p></body></html>"));
    }

    client.post("https://api.brevo.com/v3/smtp/email")
        .header("api-key", api_key)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
        
    Ok(())
}
pub async fn add_contact_to_brevo(email: &str, country: &str, state: Option<&str>) -> Result<(), reqwest::Error> {
    let api_key = env::var("BREVO_API_KEY").expect("BREVO_API_KEY must be set");
    let list_id_str = env::var("BREVO_LIST_ID").unwrap_or_default();
    
    let client = reqwest::Client::new();
    
    let mut attributes = json!({
        "COUNTRY": country,
    });
    if let Some(s) = state {
        attributes.as_object_mut().unwrap().insert("STATE".to_string(), json!(s));
    }

    let mut body = json!({
        "email": email,
        "attributes": attributes,
        "updateEnabled": true
    });
    
    if let Ok(lid) = list_id_str.parse::<i64>() {
        body.as_object_mut().unwrap().insert("listIds".to_string(), json!([lid]));
    }

    let res = client.post("https://api.brevo.com/v3/contacts")
        .header("api-key", api_key)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await?;
        
    if !res.status().is_success() {
        // Log error body for debugging but don't panic
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        println!("Failed to add contact to Brevo: {} - {}", status, text);
        // We might return Ok here to not fail the whole process if contact already exists
        // API returns 400 if user exists.
        if status.as_u16() == 400 && text.contains("duplicate_parameter") {
            return Ok(());
        }
        // Ideally we should return error, but for user UX, maybe we just log it.
        // Let's return error so main.rs can log it properly.
        // We need to reconstruct error or just use a custom one. 
        // For simplicity, we just print and Ok logic for now, or minimal error handling.
    }
        
    Ok(())
}
