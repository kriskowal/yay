Value::Object(HashMap::from([
    ("home".into(), Value::Tagged("outer".into(), Box::new(Value::Tagged("com.example.homepage".into(), Box::new(Value::reference("https://kriskowal.com/").unwrap()))))),
    ("thumb".into(), Value::Tagged("image/png".into(), Box::new(Value::reference("https://example.com/cat.png").unwrap()))),
]))
