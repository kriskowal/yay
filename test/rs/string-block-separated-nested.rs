Value::Object(HashMap::from([
    ("after".into(), Value::Integer(1.into())),
    ("outer".into(), Value::Object(HashMap::from([
        ("inner".into(), Value::Object(HashMap::from([
            ("text".into(), Value::String("Two levels in.\n".into())),
        ]))),
        ("sibling".into(), Value::String("One level in.\n".into())),
    ]))),
]))
