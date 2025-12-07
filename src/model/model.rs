use serde::Serialize;

pub mod Model {
    use std::error::Error;

    use reqwest::Request;
    use serde::Serialize;


    #[derive(Debug , Clone)]
    pub enum Role {
        User, 
        Assistant,
        System,
    }

    #[derive(Debug, Clone)]
    pub struct Model{
        pub model_name: String,
        pub api_key: String, 
        pub base_url: String,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct Message{
        pub role:  Role,
        pub content: String,
    }

    impl Message {
        pub fn new(role: Role, content: String) -> Self {
            Self { role, content }
        }
    }

    impl Serialize for Role {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer {
            serializer.serialize_str(role_to_string(self).as_ref())
        }
    }

    impl Model {
        pub fn new(model_name: String, api_key: String, base_url: String) -> Self {
            Self {
                model_name,
                api_key,
                base_url,
            }
        }

        pub async fn complete(&self, messages: Vec<Message>) -> Result<String , Box<dyn Error>>{

            let client = reqwest::Client::new();
            let mut req_builder = client.request(reqwest::Method::GET, self.base_url.clone());

            let body = request_body{
                model: self.model_name.clone(),
                messages: messages.clone()
            };

            let json_body = serde_json::to_vec(&body).unwrap();

            req_builder = req_builder
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .body(json_body);

            
            let response = req_builder.send().await?;
            dbg!(response);

            todo!("implement the chat completion logic")
        }
    }

    fn role_to_string(r: &Role) -> String{
        match  r{
            Role::User => "user".to_string(),
            Role::Assistant => "assistant".to_string(),
            Role::System => "system".to_string()
        }
    }

    #[derive(Serialize)]
    struct request_body {
        model : String,
        messages: Vec<Message>,
    } 



}


#[cfg(test)]
mod tests{
    use super::*;

}