Value::Object(HashMap::from([
    ("home".into(), Value::Tagged("outer".into(), Box::new(Value::Tagged("com.example.homepage".into(), Box::new(Value::Reference("https://kriskowal.com/".into())))))),
    ("thumb".into(), Value::Tagged("image/png".into(), Box::new(Value::Reference("https://example.com/cat.png".into())))),
]))
