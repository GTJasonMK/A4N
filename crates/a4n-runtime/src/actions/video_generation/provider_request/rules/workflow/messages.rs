use crate::actions::video_generation::{BoundaryFrameImageIds, boundary_frame_numbers_for_prompt};
use a4n_core::models::chapter::chapter_video_prompt;

pub(in crate::actions::video_generation) fn first_frame_chain_missing_start_image_message(
    prompt: &chapter_video_prompt::Model,
) -> String {
    if prompt.shot_number <= 1 {
        "首帧串联第 1 个视频单元缺少首帧图片，请先生成首帧提示词对应的首帧图片".to_string()
    } else {
        format!(
            "首帧串联第 {} 个视频单元缺少上一视频单元尾帧图片，请先生成并完成第 {} 个视频单元，让系统采集尾帧后再继续",
            prompt.shot_number,
            prompt.shot_number - 1
        )
    }
}

pub(in crate::actions::video_generation) fn boundary_frame_missing_image_message(
    prompt: &chapter_video_prompt::Model,
    frame_ids: &BoundaryFrameImageIds,
) -> Option<String> {
    let (first_frame_number, last_frame_number) = match boundary_frame_numbers_for_prompt(prompt) {
        Ok(numbers) => numbers,
        Err(message) => return Some(message),
    };
    let mut missing = Vec::new();
    if frame_ids.first.is_none() {
        missing.push(format!("第 {first_frame_number} 张首帧边界图"));
    }
    if frame_ids.last.is_none() {
        missing.push(format!("第 {last_frame_number} 张尾帧边界图"));
    }
    if missing.is_empty() {
        return None;
    }
    Some(format!(
        "首尾帧模式第 {} 个视频单元缺少{}，请先在首尾帧生成里生成对应边界帧图片",
        prompt.shot_number,
        missing.join("、")
    ))
}

pub(in crate::actions::video_generation) fn storyboard_missing_image_message(
    prompt: &chapter_video_prompt::Model,
) -> String {
    format!(
        "分镜模式第 {} 个视频单元缺少分镜图，请先生成分镜提示词对应的分镜图片",
        prompt.shot_number
    )
}
