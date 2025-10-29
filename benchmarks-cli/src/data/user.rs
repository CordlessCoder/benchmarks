use crate::data::{DataProvider, DataRow};
use benchmarks_sysinfo::user::UserData;
use owo_colors::Style;

#[derive(Debug)]
pub struct UserInfoProvider;
impl DataProvider for UserInfoProvider {
    fn identifier(&self) -> &'static str {
        "User"
    }
    fn try_fetch(&self) -> Result<Vec<super::DataRow>, String> {
        let info = UserData::fetch().map_err(|err| err.to_string())?;
        Ok(vec![
            DataRow::new("Username").with_value(info.username, Style::new()),
            DataRow::new("Home").with_value(info.home.display().to_string(), Style::new()),
            {
                let mut shell =
                    DataRow::new("Shell").with_value(info.shell.name().to_string(), Style::new());
                if let Some(version) = info.shell.version() {
                    shell.push_value(format!(" version {version}"), Style::new());
                }
                shell
            },
        ])
    }
}
