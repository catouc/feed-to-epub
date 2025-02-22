#[macro_use] extern crate lazy_static;
extern crate pwd;
extern crate dirs;

use std::{
    io,
    path::{PathBuf, MAIN_SEPARATOR}
};

use pwd::Passwd;

lazy_static! {
static ref PREFIX: String = format!("~{}", MAIN_SEPARATOR);
}

/// Takes a string-like thing and tries to turn it into a PathBuf while expanding `~`'s and `~user`'s
/// into the user's home directory
///
/// # Example
///
/// ```rust
/// extern crate expanduser;
///
/// use expanduser::expanduser;
///
/// # fn main() -> ::std::io::Result<()> {
/// # let old_home = ::std::env::var("HOME").expect("no HOME set");
/// # ::std::env::set_var("HOME", "/home/foo");
/// let path = expanduser("~/path/to/directory")?;
/// # ::std::env::set_var("HOME", &old_home);
/// assert_eq!(path.display().to_string(), "/home/foo/path/to/directory");
/// #   Ok(())
/// # }
/// ```
pub fn expanduser<S: AsRef<str>>(s: S) -> io::Result<PathBuf> {
    _expand_user(s.as_ref())
}

fn _expand_user(s: &str) -> io::Result<PathBuf> {
    Ok(match s {
        // matches an exact "~"
        s if s == "~" => {
            home_dir()?
        },
        // matches paths that start with `~/`
        s if s.starts_with(&*PREFIX) => {
            let home = home_dir()?;
            home.join(&s[2..])
        },
        // matches paths that start with `~` but not `~/`, might be a `~username/` path
        s if s.starts_with("~") => {
            let mut parts = s[1..].splitn(2, MAIN_SEPARATOR);
            let user = parts.next()
                            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "malformed path"))?;
            let user = Passwd::from_name(&user)
                              .map_err(|_| io::Error::new(io::ErrorKind::Other, "error searching for user"))?
                              .ok_or_else(|| io::Error::new(io::ErrorKind::Other, format!("user '{}', does not exist", &user)))?;
            if let Some(ref path) = parts.next() {
                PathBuf::from(user.dir).join(&path)
            } else {
                PathBuf::from(user.dir)
            }
        },
        // nothing to expand, just make a PathBuf
        s => PathBuf::from(s)
    })
}

fn home_dir() -> io::Result<PathBuf> {
    dirs::home_dir().ok_or_else(|| io::Error::new(io::ErrorKind::Other, "no home directory is set"))
}

#[cfg(test)]
mod tests {
    use std::env;
    use super::*;

    // Until I figure out a better to way to test this stuff in isolation, it is necessary to run
    // this using `cargo test -- --test-threads 1`, otherwise you will probably get race conditions
    // from the HOME manipulation

    #[test]
    fn test_success() {
        let old_home = env::var("HOME").expect("no home dir set");
        let new_home = "/home/foo";
        env::set_var("HOME", new_home);
        let path = expanduser("~/path/to/directory");
        env::set_var("HOME", old_home);
        assert_eq!(path.expect("io error"), PathBuf::from("/home/foo/path/to/directory"));
    }

    #[test]
    fn test_only_tilde() {
        let old_home = env::var("HOME").expect("no home dir set");
        let new_home = "/home/foo";
        env::set_var("HOME", new_home);
        let pathstr = "~";
        let path = expanduser(pathstr);
        env::set_var("HOME", old_home);
        assert_eq!(path.expect("io error"), PathBuf::from("/home/foo"));
    }

    #[test]
    fn test_user() {
        let user = env::var("USER").expect("no user set");
        if user.len() < 1 {
            panic!("user is empty");
        }
        let home = dirs::home_dir().expect("no home directory set");
        let pathstr = format!("~{}/path/to/directory", &user);
        let path = expanduser(&pathstr).expect("io error");
        assert_eq!(path, home.join("path/to/directory"));
    }

    #[test]
    fn test_just_tilde_user() {
        let user = env::var("USER").expect("no user set");
        if user.len() < 1 {
            panic!("user is empty");
        }
        let home = dirs::home_dir().expect("no home directory set");
        let pathstr = format!("~{}", &user);
        let path = expanduser(&pathstr).expect("io error");
        assert_eq!(path, home);
    }

    #[test]
    fn test_fail_malformed_path() {
        let pathstr = "~\ruses-invalid-path-char";
        let err = expanduser(&pathstr).unwrap_err();
        let kind = err.kind();
        assert_eq!(kind, io::ErrorKind::Other);
    }

    #[test]
    #[should_panic]
    fn test_user_does_not_exist() {
        expanduser("~user_that_should_not_exist/path/to/directory")
                        .expect("user does not exist");
    }
}
