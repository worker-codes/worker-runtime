
// use anyhow::Result;
use urlpattern::quirks;
use urlpattern::quirks::MatchInput;
use urlpattern::quirks::StringOrInit;
use urlpattern::quirks::UrlPattern;

use crate::error::AnyError;
use crate::error::type_error;

#[derive(Deserialize, Serialize, Debug)]
struct URLPatternTestResponse {
    value: bool,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UrlPatternRequest {
    pattern: ByteBuf,
    test: ByteBuf,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct UrlPatternInitArg {
    pub protocol: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub hostname: Option<String>,
    pub port: Option<String>,
    pub pathname: Option<String>,
    pub search: Option<String>,
    pub hash: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UrlPatternResultArg {
    pub protocol: UrlPatternComponentResultArg,
    pub username: UrlPatternComponentResultArg,
    pub password: UrlPatternComponentResultArg,
    pub hostname: UrlPatternComponentResultArg,
    pub port: UrlPatternComponentResultArg,
    pub pathname: UrlPatternComponentResultArg,
    pub search: UrlPatternComponentResultArg,
    pub hash: UrlPatternComponentResultArg,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UrlPatternComponentResultArg {
    pub input: String,
    pub groups: HashMap<String, String>,
}

pub fn urlpattern_parse(
  input: StringOrInit,
  base_url: Option<String>,
) -> Result<UrlPattern, AnyError> {
  let init = urlpattern::quirks::process_construct_pattern_input(
    input,
    base_url.as_deref(),
  )
  .map_err(|e| type_error(e.to_string()))?;

  let pattern = urlpattern::quirks::parse_pattern(init)
    .map_err(|e| type_error(e.to_string()))?;

  Ok(pattern)
}






pub fn urlpattern_process_match_input(
  input: StringOrInit,
  base_url: Option<String>,
) -> Result<Option<(MatchInput, quirks::Inputs)>, AnyError> {
  let res = urlpattern::quirks::process_match_input(input, base_url.as_deref())
    .map_err(|e| type_error(e.to_string()))?;

  let (input, inputs) = match res {
    Some((input, inputs)) => (input, inputs),
    None => return Ok(None),
  };

  Ok(urlpattern::quirks::parse_match_input(input).map(|input| (input, inputs)))
}