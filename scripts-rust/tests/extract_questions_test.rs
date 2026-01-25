use claude_hooks::extract_questions_with_options;
use serde_json::json;

#[test]
fn test_extract_questions_with_options_single_question() {
    let tool_input = json!({
        "questions": [
            {
                "question": "Which library should we use?",
                "header": "Library",
                "options": [
                    {"label": "Option A", "description": "Fast and simple"},
                    {"label": "Option B", "description": "Full featured"}
                ]
            }
        ]
    });

    let result = extract_questions_with_options(&tool_input);
    assert!(result.contains("*Q1: [Library]* Which library should we use?"));
    assert!(result.contains("1. Option A - Fast and simple"));
    assert!(result.contains("2. Option B - Full featured"));
}

#[test]
fn test_extract_questions_with_options_multiple_questions() {
    let tool_input = json!({
        "questions": [
            {
                "question": "First question?",
                "header": "Q1",
                "options": [
                    {"label": "Yes", "description": "Agree"},
                    {"label": "No", "description": "Disagree"}
                ]
            },
            {
                "question": "Second question?",
                "header": "Q2",
                "options": [
                    {"label": "A"},
                    {"label": "B"}
                ]
            }
        ]
    });

    let result = extract_questions_with_options(&tool_input);
    assert!(result.contains("*Q1: [Q1]* First question?"));
    assert!(result.contains("*Q2: [Q2]* Second question?"));
}

#[test]
fn test_extract_questions_with_options_no_header() {
    let tool_input = json!({
        "questions": [
            {
                "question": "Simple question?",
                "options": [
                    {"label": "Yes"},
                    {"label": "No"}
                ]
            }
        ]
    });

    let result = extract_questions_with_options(&tool_input);
    assert!(result.contains("*Q1:* Simple question?"));
}

#[test]
fn test_extract_questions_with_options_no_questions() {
    let tool_input = json!({});
    let result = extract_questions_with_options(&tool_input);
    assert_eq!(result, "N/A");
}

#[test]
fn test_extract_questions_with_options_empty_questions() {
    let tool_input = json!({
        "questions": []
    });
    let result = extract_questions_with_options(&tool_input);
    assert_eq!(result, "");
}

#[test]
fn test_extract_questions_option_without_description() {
    let tool_input = json!({
        "questions": [
            {
                "question": "Pick one?",
                "header": "Choice",
                "options": [
                    {"label": "Alpha"},
                    {"label": "Beta"}
                ]
            }
        ]
    });

    let result = extract_questions_with_options(&tool_input);
    assert!(result.contains("1. Alpha"));
    assert!(result.contains("2. Beta"));
    assert!(!result.contains(" - ")); // No description separator
}
