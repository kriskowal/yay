Value::Array(vec![Value::Object(HashMap::from([(
    "nested".into(),
    Value::Object(HashMap::from([("deep".into(), Value::Integer(42.into()))])),
)]))])
