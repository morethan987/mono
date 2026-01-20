use crate::error::{MonoError, Result};
use crate::protocol::{Request, Response};

pub fn encode_request(request: &Request) -> Result<String> {
    let json = serde_json::to_string(request)?;
    Ok(format!("{}\n", json))
}

pub fn decode_request(data: &str) -> Result<Request> {
    let trimmed = data.trim();
    if trimmed.is_empty() {
        return Err(MonoError::IpcProtocol {
            message: "Empty request".to_string(),
        });
    }
    serde_json::from_str(trimmed).map_err(|e| MonoError::IpcProtocol {
        message: format!("Invalid request JSON: {}", e),
    })
}

pub fn encode_response(response: &Response) -> Result<String> {
    let json = serde_json::to_string(response)?;
    Ok(format!("{}\n", json))
}

pub fn decode_response(data: &str) -> Result<Response> {
    let trimmed = data.trim();
    if trimmed.is_empty() {
        return Err(MonoError::IpcProtocol {
            message: "Empty response".to_string(),
        });
    }
    serde_json::from_str(trimmed).map_err(|e| MonoError::IpcProtocol {
        message: format!("Invalid response JSON: {}", e),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_roundtrip() {
        let request = Request::AddTask {
            title: "Test task".to_string(),
            description: None,
            priority: None,
            tags: vec!["work".to_string()],
            estimated_minutes: Some(30),
            deadline: None,
        };

        let encoded = encode_request(&request).unwrap();
        assert!(encoded.ends_with('\n'));

        let decoded = decode_request(&encoded).unwrap();
        assert!(matches!(decoded, Request::AddTask { title, .. } if title == "Test task"));
    }

    #[test]
    fn test_response_roundtrip() {
        let response = Response::error("Something went wrong");

        let encoded = encode_response(&response).unwrap();
        let decoded = decode_response(&encoded).unwrap();

        assert!(
            matches!(decoded, Response::Error { message } if message == "Something went wrong")
        );
    }
}
