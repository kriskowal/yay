Value::Object(HashMap::from([
    ("a".into(), Value::Tagged("utc".into(), Box::new(Value::Tagged("datetime".into(), Box::new(Value::String("2026-11-01T15:00:00".into())))))),
    ("b".into(), Value::Tagged("outer".into(), Box::new(Value::Tagged("inner".into(), Box::new(Value::Array(vec![
        Value::Integer(1.into()),
        Value::Integer(2.into()),
    ])))))),
]))
