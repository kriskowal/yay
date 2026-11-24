Value::Array(vec![
    Value::Object(HashMap::from([
        ("color".into(), Value::Tagged("rgb".into(), Box::new(Value::Array(vec![Value::Integer(1.into())])))),
        ("name".into(), Value::String("a".into())),
    ])),
])
