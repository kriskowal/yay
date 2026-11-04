Value::Object(HashMap::from([
    ("peers".into(), Value::Array(vec![
        Value::Tagged("Counter".into(), Box::new(Value::Reference("ocapn://a.tcp".into()))),
        Value::Reference("ocapn://b.tcp".into()),
    ])),
]))
