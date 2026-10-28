use std::fs;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use super::*;

struct Harness {
    service: ProjectService,
    trashed: Arc<Mutex<Vec<PathBuf>>>,
    dir: tempfile::TempDir,
}

impl Harness {
    async fn new() -> Self {
        let trashed = Arc::new(Mutex::new(Vec::new()));
        let log = trashed.clone();
        let service =
            ProjectService::with_trash(Store::open_in_memory().await.unwrap(), move |p| {
                log.lock().unwrap().push(p.to_path_buf());
                Ok(())
            });
        Self {
            service,
            trashed,
            dir: tempfile::tempdir().unwrap(),
        }
    }

    /// Canonical temp root (on macOS/Windows temp paths may contain links).
    fn root(&self) -> PathBuf {
        dunce::canonicalize(self.dir.path()).unwrap()
    }

    fn path(&self, rel: &str) -> String {
        self.root().join(rel).to_str().unwrap().to_owned()
    }
}

fn create_req(location: String, template: Option<&str>) -> CreateProjectRequest {
    CreateProjectRequest {
        name: "Web Platform".into(),
        description: "Multi-server dev env".into(),
        location,
        repository_url: Some("https://github.com/o/r.git".into()),
        tags: vec!["Web".into()],
        template_id: template.map(str::to_owned),
    }
}

/// (relative path, size, mtime) for every entry, sorted — detects any change.
fn snapshot(root: &Path) -> Vec<(String, u64, SystemTime)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap().flatten() {
            // fs::metadata, not DirEntry::metadata: on Windows the latter returns
            // cached directory-index data that is updated lazily.
            let meta = fs::metadata(entry.path()).unwrap();
            out.push((
                entry
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .display()
                    .to_string(),
                meta.len(),
                meta.modified().unwrap(),
            ));
            if meta.is_dir() {
                stack.push(entry.path());
            }
        }
    }
    out.sort();
    out
}

#[tokio::test]
async fn create_with_template_writes_vagrantfile_and_registers_environment() {
    let h = Harness::new().await;
    let details = h
        .service
        .create(create_req(h.path(""), Some("web-database")))
        .await
        .unwrap();
    let folder = h.root().join("web-platform");
    assert_eq!(details.project.root_path, folder.to_str().unwrap());
    assert_eq!(details.project.tags, vec!["web"]);
    let vagrantfile = fs::read_to_string(folder.join("Vagrantfile")).unwrap();
    assert_eq!(
        vagrantfile,
        templates::find("web-database").unwrap().vagrantfile
    );
    assert_eq!(details.environments.len(), 1);
    assert_eq!(details.environments[0].name, "default");
    assert_eq!(
        details.environments[0].vagrantfile_path.as_deref(),
        Some("Vagrantfile")
    );
}

#[tokio::test]
async fn create_without_template_makes_an_empty_project_folder() {
    let h = Harness::new().await;
    let details = h
        .service
        .create(create_req(h.path(""), None))
        .await
        .unwrap();
    assert!(details.environments.is_empty());
    assert!(is_empty_dir(Path::new(&details.project.root_path)).unwrap());
}

#[tokio::test]
async fn create_never_uses_a_non_empty_folder() {
    let h = Harness::new().await;
    fs::create_dir(h.root().join("web-platform")).unwrap();
    fs::write(h.root().join("web-platform/Vagrantfile"), "user content").unwrap();
    let err = h
        .service
        .create(create_req(h.path(""), Some("single-linux-vm")))
        .await
        .unwrap_err();
    assert!(matches!(err, ServiceError::Conflict(_)), "{err}");
    assert_eq!(
        fs::read_to_string(h.root().join("web-platform/Vagrantfile")).unwrap(),
        "user content"
    );
}

#[tokio::test]
async fn create_rejects_bad_input_before_touching_disk() {
    let h = Harness::new().await;
    let mut req = create_req(h.path(""), Some("nope"));
    assert!(matches!(
        h.service.create(req.clone()).await,
        Err(ServiceError::Validation(_))
    ));
    req.template_id = None;
    req.location = "relative/path".into();
    assert!(matches!(
        h.service.create(req).await,
        Err(ServiceError::Validation(_))
    ));
    assert!(snapshot(&h.root()).is_empty());
}

#[tokio::test]
async fn import_does_not_modify_files() {
    let h = Harness::new().await;
    let root = h.root().join("existing project");
    fs::create_dir_all(root.join("envs/test")).unwrap();
    fs::write(root.join("Vagrantfile"), "Vagrant.configure(\"2\") {}").unwrap();
    fs::write(root.join("envs/test/Vagrantfile"), "x").unwrap();
    fs::create_dir_all(root.join(".vagrant-studio")).unwrap();
    fs::write(
        root.join(".vagrant-studio/project.json"),
        r#"{"schemaVersion":1,"project":{"name":"From Metadata"}}"#,
    )
    .unwrap();
    let before = snapshot(&root);

    let inspection = h
        .service
        .inspect_folder(root.to_str().unwrap())
        .await
        .unwrap();
    assert_eq!(inspection.suggested_name, "From Metadata");
    assert_eq!(inspection.environments.len(), 2);

    let details = h
        .service
        .import(ImportProjectRequest {
            root_path: inspection.path.clone(),
            name: inspection.suggested_name,
            description: String::new(),
            repository_url: None,
            tags: vec![],
            environments: inspection
                .environments
                .iter()
                .map(|e| EnvironmentRequest {
                    name: e.name.clone(),
                    relative_path: e.relative_path.clone(),
                    environment_type: None,
                })
                .collect(),
        })
        .await
        .unwrap();

    assert_eq!(details.environments.len(), 2);
    assert_eq!(snapshot(&root), before, "import changed files on disk");

    let again = h
        .service
        .inspect_folder(root.to_str().unwrap())
        .await
        .unwrap();
    assert_eq!(again.existing_project_id, Some(details.project.id));
    assert_eq!(snapshot(&root), before);
}

#[tokio::test]
async fn import_rejects_environments_outside_the_project() {
    let h = Harness::new().await;
    fs::create_dir_all(h.root().join("proj")).unwrap();
    fs::create_dir_all(h.root().join("other")).unwrap();
    for bad in ["../other", "missing", "/etc"] {
        let err = h
            .service
            .import(ImportProjectRequest {
                root_path: h.path("proj"),
                name: "P".into(),
                description: String::new(),
                repository_url: None,
                tags: vec![],
                environments: vec![EnvironmentRequest {
                    name: "e".into(),
                    relative_path: bad.into(),
                    environment_type: None,
                }],
            })
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::Validation(_)), "{bad}: {err}");
    }
    assert!(h.service.list(ProjectFilter::All).await.unwrap().is_empty());
}

#[tokio::test]
async fn delete_metadata_keeps_files() {
    let h = Harness::new().await;
    let details = h
        .service
        .create(create_req(h.path(""), Some("single-linux-vm")))
        .await
        .unwrap();
    let outcome = h
        .service
        .delete(details.project.id, DeleteProjectRequest::default())
        .await
        .unwrap();
    assert!(!outcome.files_moved_to_trash);
    assert!(
        Path::new(&details.project.root_path)
            .join("Vagrantfile")
            .is_file()
    );
    assert!(h.trashed.lock().unwrap().is_empty());
    assert!(matches!(
        h.service.details(details.project.id).await,
        Err(ServiceError::NotFound(_))
    ));
}

#[tokio::test]
async fn deleting_files_requires_typed_name_and_no_vagrant_machines() {
    let h = Harness::new().await;
    let details = h
        .service
        .create(create_req(h.path(""), Some("single-linux-vm")))
        .await
        .unwrap();
    let id = details.project.id;
    let root = PathBuf::from(&details.project.root_path);

    let wrong = h
        .service
        .delete(
            id,
            DeleteProjectRequest {
                delete_files: true,
                confirm_name: Some("web platform".into()),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(wrong, ServiceError::Validation(_)));

    fs::create_dir_all(root.join(".vagrant/machines/default/virtualbox")).unwrap();
    fs::write(root.join(".vagrant/machines/default/virtualbox/id"), "uuid").unwrap();
    let unsafe_err = h
        .service
        .delete(
            id,
            DeleteProjectRequest {
                delete_files: true,
                confirm_name: Some("Web Platform".into()),
            },
        )
        .await
        .unwrap_err();
    assert!(
        matches!(unsafe_err, ServiceError::Unsafe(_)),
        "{unsafe_err}"
    );
    assert!(h.trashed.lock().unwrap().is_empty());
    assert!(
        h.service.details(id).await.is_ok(),
        "metadata kept on refusal"
    );

    fs::remove_dir_all(root.join(".vagrant")).unwrap();
    let outcome = h
        .service
        .delete(
            id,
            DeleteProjectRequest {
                delete_files: true,
                confirm_name: Some(" Web Platform ".into()),
            },
        )
        .await
        .unwrap();
    assert!(outcome.files_moved_to_trash);
    assert_eq!(*h.trashed.lock().unwrap(), vec![root]);
}

#[test]
fn trash_refuses_top_level_folders() {
    assert!(ensure_safe_to_trash(Path::new("/")).is_err());
    assert!(ensure_safe_to_trash(Path::new("/home")).is_err());
    if let Some(home) = std::env::var_os("HOME").and_then(|h| dunce::canonicalize(h).ok()) {
        assert!(ensure_safe_to_trash(&home).is_err());
    }
    assert!(ensure_safe_to_trash(Path::new("/home/u/projects/app")).is_ok());
}

#[tokio::test]
async fn portable_metadata_is_previewed_and_never_silently_overwritten() {
    let h = Harness::new().await;
    let details = h
        .service
        .create(create_req(h.path(""), Some("single-linux-vm")))
        .await
        .unwrap();
    let id = details.project.id;

    let preview = h.service.preview_portable(id).await.unwrap();
    assert_eq!(preview.existing, None);
    let parsed = PortableProject::parse(&preview.content).unwrap();
    assert_eq!(parsed.project.name, "Web Platform");
    assert_eq!(parsed.environments[0].path, ".");
    assert!(!preview.content.contains(&h.path("")), "no absolute paths");

    h.service.write_portable(id, false).await.unwrap();
    assert_eq!(fs::read_to_string(&preview.path).unwrap(), preview.content);

    fs::write(&preview.path, "{\"edited\": true}").unwrap();
    let err = h.service.write_portable(id, false).await.unwrap_err();
    assert!(
        matches!(&err, ServiceError::ConfirmationRequired { existing: Some(e), .. } if e.contains("edited"))
    );
    assert_eq!(
        fs::read_to_string(&preview.path).unwrap(),
        "{\"edited\": true}"
    );

    h.service.write_portable(id, true).await.unwrap();
    assert_eq!(fs::read_to_string(&preview.path).unwrap(), preview.content);
}

#[tokio::test]
async fn update_validates_and_settings_round_trip() {
    let h = Harness::new().await;
    let details = h
        .service
        .create(create_req(h.path(""), None))
        .await
        .unwrap();
    let id = details.project.id;
    let bad = h
        .service
        .update(
            id,
            ProjectChanges {
                repository_url: Some(Some("https://user:pw@h/r".into())),
                ..Default::default()
            },
        )
        .await;
    assert!(matches!(bad, Err(ServiceError::Validation(_))));

    let settings = AppSettings {
        default_project_location: Some(h.path("")),
    };
    h.service.save_settings(settings.clone()).await.unwrap();
    assert_eq!(h.service.settings().await.unwrap(), settings);
    assert!(
        h.service
            .save_settings(AppSettings {
                default_project_location: Some("/definitely/missing".into())
            })
            .await
            .is_err()
    );
}

#[tokio::test]
async fn projects_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("studio.db");
    let projects = dir.path().join("projects");
    fs::create_dir(&projects).unwrap();
    let location = dunce::canonicalize(&projects).unwrap();

    let store = Store::open(&db).await.unwrap();
    let service = ProjectService::new(store.clone());
    let created = service
        .create(create_req(
            location.to_str().unwrap().into(),
            Some("web-database"),
        ))
        .await
        .unwrap();
    service
        .set_favorite(created.project.id, true)
        .await
        .unwrap();
    store.close().await;
    drop(service);

    let service = ProjectService::new(Store::open(&db).await.unwrap());
    let reopened = service.details(created.project.id).await.unwrap();
    assert_eq!(reopened.project.name, "Web Platform");
    assert!(reopened.project.favorite);
    assert_eq!(reopened.environments, created.environments);
}

#[tokio::test]
async fn environments_can_be_added_by_absolute_path_inside_the_project_only() {
    let h = Harness::new().await;
    let details = h
        .service
        .create(create_req(h.path(""), None))
        .await
        .unwrap();
    let root = PathBuf::from(&details.project.root_path);
    fs::create_dir_all(root.join("envs/qa")).unwrap();
    fs::write(root.join("envs/qa/Vagrantfile"), "").unwrap();

    let env = h
        .service
        .add_environment(
            details.project.id,
            EnvironmentRequest {
                name: "qa".into(),
                relative_path: root.join("envs/qa").to_str().unwrap().into(),
                environment_type: Some(EnvironmentType::Testing),
            },
        )
        .await
        .unwrap();
    assert_eq!(env.vagrantfile_path.as_deref(), Some("Vagrantfile"));
    assert_eq!(env.environment_type, EnvironmentType::Testing);

    let outside = h
        .service
        .add_environment(
            details.project.id,
            EnvironmentRequest {
                name: "other".into(),
                relative_path: h.path(""),
                environment_type: None,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(outside, ServiceError::Validation(_)));

    h.service.remove_environment(env.id).await.unwrap();
    assert!(
        root.join("envs/qa/Vagrantfile").is_file(),
        "files untouched"
    );
    assert!(matches!(
        h.service.remove_environment(env.id).await,
        Err(ServiceError::NotFound(_))
    ));
}
