Value::Object(HashMap::from([
    ("".into(), Value::String("the empty key".into())),
    ("a\"b".into(), Value::String("an escaped quote".into())),
    ("a/b".into(), Value::String("a slash".into())),
    ("a\\b".into(), Value::String("an escaped backslash".into())),
    ("line\nbreak".into(), Value::String("an escaped newline".into())),
]))
