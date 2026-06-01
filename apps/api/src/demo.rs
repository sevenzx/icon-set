use crate::models::{IconEntry, IconManifest, IconSetSummary};

const UPDATED_AT: &str = "2026-06-01T00:00:00Z";

pub fn list_sets() -> Vec<IconSetSummary> {
    demo_manifests()
        .into_iter()
        .map(|manifest| IconSetSummary {
            id: manifest.id,
            name: manifest.name,
            description: manifest.description,
            icon_count: manifest.icons.len(),
            updated_at: UPDATED_AT.to_string(),
        })
        .collect()
}

pub fn get_set(set_id: &str) -> Option<IconManifest> {
    demo_manifests()
        .into_iter()
        .find(|manifest| manifest.id == set_id)
}

fn demo_manifests() -> Vec<IconManifest> {
    vec![
        IconManifest {
            id: "media".to_string(),
            name: "媒体图标演示".to_string(),
            description: "内置演示数据，不关联真实 GitHub 仓库。".to_string(),
            icons: vec![
                demo_icon("demo-media-01", "neon_play", "media", "neon-play"),
                demo_icon("demo-media-02", "stream_box", "media", "stream-box"),
                demo_icon("demo-media-03", "wave_cast", "media", "wave-cast"),
            ],
            updated_at: UPDATED_AT.to_string(),
        },
        IconManifest {
            id: "service".to_string(),
            name: "服务图标演示".to_string(),
            description: "用于展示搜索、预览和复制链接的假数据集合。".to_string(),
            icons: vec![
                demo_icon("demo-service-01", "cloud_node", "service", "cloud-node"),
                demo_icon("demo-service-02", "orbit_db", "service", "orbit-db"),
                demo_icon("demo-service-03", "pixel_api", "service", "pixel-api"),
            ],
            updated_at: UPDATED_AT.to_string(),
        },
    ]
}

fn demo_icon(id: &str, name: &str, set_id: &str, slug: &str) -> IconEntry {
    let path = format!("demo/{set_id}/icons/{slug}.svg");
    let url = "/demo-icons/icon-set.png".to_string();

    IconEntry {
        id: id.to_string(),
        name: name.to_string(),
        path,
        url,
        md5: String::new(),
    }
}
