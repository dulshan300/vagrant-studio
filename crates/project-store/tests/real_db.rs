//! Replays the app's read and write paths against a copy of a real database.
//! `VS_DB=<copy of vagrant-studio.db> cargo test -p vagrant-studio-project-store --test real_db -- --ignored --nocapture`

use std::path::Path;

use vagrant_studio_project_store::{ProjectFilter, Store, TaskUpdate};

#[tokio::test]
#[ignore = "needs a copy of a real database in VS_DB"]
async fn replays_app_queries() {
    let path = std::env::var("VS_DB").expect("VS_DB");
    let store = Store::open(Path::new(&path)).await.unwrap();
    println!("open ok");
    let projects = store.list_projects(ProjectFilter::All).await.unwrap();
    println!("projects {}", projects.len());
    for p in &projects {
        let envs = store.list_environments(p.id).await.unwrap();
        println!("envs {}", envs.len());
        for e in &envs {
            println!(
                "status {:?}",
                store
                    .get_environment_status(e.id)
                    .await
                    .map(|s| s.is_some())
            );
            println!(
                "latest prov {:?}",
                store.latest_provisioning(e.id).await.map(|r| r.len())
            );
            println!(
                "ops {:?}",
                store.list_operations(Some(e.id), 50).await.map(|o| o.len())
            );
        }
        println!("notes {:?}", store.list_notes(p.id).await.map(|n| n.len()));
        let task = store.insert_task(p.id, "probe").await.unwrap();
        println!("task inserted");
        store
            .update_task(
                task.id,
                &TaskUpdate {
                    done: Some(true),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        println!("task updated");
        store.reorder_tasks(p.id, &[task.id]).await.unwrap();
        store.delete_task(task.id).await.unwrap();
        let note = store.insert_note(p.id, "probe", "x").await.unwrap();
        store.update_note(note.id, "probe", "y").await.unwrap();
        store.delete_note(note.id).await.unwrap();
        println!("notes/tasks ok");
    }
    println!("tags {:?}", store.list_tags().await.map(|t| t.len()));
    println!(
        "all ops {:?}",
        store.list_operations(None, 200).await.map(|o| o.len())
    );
    println!(
        "statuses {:?}",
        store.list_environment_statuses().await.map(|s| s.len())
    );
}
