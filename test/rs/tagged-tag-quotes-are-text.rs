Value::Object(HashMap::from([
    ("a".into(), Value::Tagged("'a'".into(), Box::new(Value::Integer(1.into())))),
    ("b".into(), Value::Tagged("\"\"".into(), Box::new(Value::Integer(2.into())))),
]))
