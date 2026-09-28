pub const MAX_ARGS: usize = 16;
pub const MAX_ARG_LEN: usize = 128;

pub struct ParseCommand<'a> {
    pub argc: usize,
    pub argv: [&'a [u8]; MAX_ARGS],
}

impl <'a> ParseCommand<'a> {
    pub fn empty() -> Self {
        Self { argc: 0, argv: [&[]; MAX_ARGS] }
    }

    pub fn command(&self) -> Option<&[u8]> {
        if self.argc == 0 {
            None
        } else {
            Some(self.argv[0])
        }
    }

    pub fn arg(&self, index: usize) -> Option<&[u8]> {
        if index < self.argc {
            Some(self.argv[index])
        } else {
            None
        }
    }
}

pub fn parse(line: &[u8]) -> ParseCommand<'_> {
    let mut result = ParseCommand::empty();

    let mut i = 0;

    while i < line.len() {
        while i < line.len() && line[i].is_ascii_whitespace() {
            i += 1;
        }

        if i >= line.len() {
            break;
        }

        let start = i;

        while i < line.len() && !line[i].is_ascii_whitespace() {
            i += 1;
        }

        if result.argc >= MAX_ARGS {
            break;
        }

        result.argv[result.argc] = &line[start..i];
        result.argc += 1;
    }

   result
}
