Value::Object(HashMap::from([
    ("peers".into(), Value::Array(vec![
        Value::Tagged("Counter".into(), Box::new(Value::reference("ocapn://a.tcp").unwrap())),
        Value::reference("ocapn://b.tcp").unwrap(),
    ])),
]))
