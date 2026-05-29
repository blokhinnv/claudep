use zed_extension_api as zed;

struct ZedClaudeProxyExtension;

impl zed::Extension for ZedClaudeProxyExtension {
    fn new() -> Self {
        Self
    }
}

zed::register_extension!(ZedClaudeProxyExtension);
