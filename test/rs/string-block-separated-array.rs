Value::Object(HashMap::from([
    ("items".into(), Value::Array(vec![
        Value::String("\nBare.\n".into()),
        Value::String("Same line.\n".into()),
        Value::Object(HashMap::from([
            ("body".into(), Value::String("Last property of a list item.\n".into())),
            ("title".into(), Value::String("a".into())),
        ])),
        Value::Object(HashMap::from([
            ("body".into(), Value::String("First property of a list item.\n".into())),
            ("title".into(), Value::String("b".into())),
        ])),
    ])),
    ("next".into(), Value::Integer(1.into())),
]))
