map[string]any{
	"home": yay.Tagged{Tag: "outer", Payload: yay.Tagged{Tag: "com.example.homepage", Payload: yay.MustReference("https://kriskowal.com/")}},
	"thumb": yay.Tagged{Tag: "image/png", Payload: yay.MustReference("https://example.com/cat.png")},
}
