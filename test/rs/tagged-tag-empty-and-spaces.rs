Value::Object(HashMap::from([
    ("a".into(), Value::Tagged("".into(), Box::new(Value::Integer(1.into())))),
    ("b".into(), Value::Tagged(" edge ".into(), Box::new(Value::Integer(2.into())))),
    ("c".into(), Value::Tagged(" ".into(), Box::new(Value::Integer(3.into())))),
]))
