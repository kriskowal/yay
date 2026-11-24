Value::Object(HashMap::from([
    ("a".into(), Value::Tagged("md".into(), Box::new(Value::String("- not an item\n".into())))),
    ("b".into(), Value::Integer(1.into())),
]))
