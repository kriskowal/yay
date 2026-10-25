Value::Array(vec![
    Value::Object(HashMap::from([
        ("body".into(), Value::String("- x\n".into())),
        ("title".into(), Value::String("a".into())),
    ])),
    Value::Object(HashMap::from([
        ("body".into(), Value::String("# Heading\n\n\n* y\n".into())),
        ("title".into(), Value::String("b".into())),
    ])),
])
