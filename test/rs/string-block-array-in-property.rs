Value::Object(HashMap::from([
    ("items".into(), Value::Array(vec![
        Value::String("a\n- b\n".into()),
        Value::String("\n* c\n\n\n---\n".into()),
    ])),
    ("next".into(), Value::Integer(1.into())),
]))
