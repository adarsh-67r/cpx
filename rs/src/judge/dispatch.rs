// Routes a problem to its platform's parsers, and builds the judge's submit page.

use crate::judge::{atcoder, codeforces, cses};
use crate::problem::{Problem, Sample};
use anyhow::{anyhow, Result};

/// Samples for p, from its platform's problem page.
pub fn fetch_samples(p: &Problem) -> Result<Vec<Sample>> {
    match p.platform.as_str() {
        "codeforces" => codeforces::fetch_samples(&p.url),
        "cses" => cses::fetch_samples(&p.url),
        "atcoder" => atcoder::fetch_samples(&p.url),
        other => Err(anyhow!("no sample fetcher for platform {other:?}")),
    }
}

/// The page where a solution is pasted and submitted by hand. None when the
/// platform or id does not map to a known page.
pub fn submit_url(p: &Problem) -> Option<String> {
    match p.platform.as_str() {
        "codeforces" => {
            // Codeforces ids are the contest number and an index letter, e.g. "4A".
            let split = p.id.find(|c: char| c.is_ascii_alphabetic())?;
            if split == 0 {
                return None;
            }
            Some(format!("https://codeforces.com/problemset/submit/{}/{}", &p.id[..split], &p.id[split..]))
        }
        "cses" => Some(format!("https://cses.fi/problemset/submit/{}", p.id)),
        "atcoder" => {
            let (contest, _) = p.id.split_once('_')?;
            Some(format!("https://atcoder.jp/contests/{contest}/submit?taskScreenName={}", p.id))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(platform: &str, id: &str) -> Problem {
        Problem { platform: platform.into(), id: id.into(), ..Default::default() }
    }

    #[test]
    fn submit_urls() {
        assert_eq!(submit_url(&p("codeforces", "4A")).unwrap(), "https://codeforces.com/problemset/submit/4/A");
        assert_eq!(submit_url(&p("codeforces", "1095F")).unwrap(), "https://codeforces.com/problemset/submit/1095/F");
        assert_eq!(submit_url(&p("cses", "1068")).unwrap(), "https://cses.fi/problemset/submit/1068");
        assert_eq!(
            submit_url(&p("atcoder", "abc001_1")).unwrap(),
            "https://atcoder.jp/contests/abc001/submit?taskScreenName=abc001_1"
        );
        assert!(submit_url(&p("codeforces", "4")).is_none());
    }
}
