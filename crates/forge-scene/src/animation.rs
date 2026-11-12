use std::collections::HashMap;

use forge_core::id::ObjectId;
use forge_math::Transform;
use serde::{Deserialize, Serialize};

/// Interpolation mode between animation keyframes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum KeyframeInterpolation {
    #[default]
    Linear,
    Constant,
    Bezier,
}

/// A transform keyframe positioned at a specific frame index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformKeyframe {
    pub frame: u32,
    pub transform: Transform,
    pub interpolation: KeyframeInterpolation,
}

impl TransformKeyframe {
    pub fn new(frame: u32, transform: Transform) -> Self {
        Self {
            frame,
            transform,
            interpolation: KeyframeInterpolation::Linear,
        }
    }

    pub fn with_interpolation(frame: u32, transform: Transform, interpolation: KeyframeInterpolation) -> Self {
        Self {
            frame,
            transform,
            interpolation,
        }
    }
}

/// Keyframe animation track for a single scene object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectAnimationTrack {
    pub object_id: ObjectId,
    pub keyframes: Vec<TransformKeyframe>,
}

impl ObjectAnimationTrack {
    pub fn new(object_id: ObjectId) -> Self {
        Self {
            object_id,
            keyframes: Vec::new(),
        }
    }

    /// Inserts or replaces a keyframe at `frame`, maintaining sorted order by frame number.
    pub fn insert_keyframe(&mut self, frame: u32, transform: Transform) {
        if let Some(existing) = self.keyframes.iter_mut().find(|k| k.frame == frame) {
            existing.transform = transform;
            return;
        }

        let keyframe = TransformKeyframe::new(frame, transform);
        let idx = self.keyframes.partition_point(|k| k.frame < frame);
        self.keyframes.insert(idx, keyframe);
    }

    /// Removes the keyframe at `frame` if it exists. Returns true if a keyframe was removed.
    pub fn remove_keyframe(&mut self, frame: u32) -> bool {
        if let Some(pos) = self.keyframes.iter().position(|k| k.frame == frame) {
            self.keyframes.remove(pos);
            true
        } else {
            false
        }
    }

    /// Returns true if a keyframe exists exactly at `frame`.
    pub fn has_keyframe(&self, frame: u32) -> bool {
        self.keyframes.iter().any(|k| k.frame == frame)
    }

    /// Evaluates the object's transform at a fractional frame with interpolation.
    pub fn evaluate(&self, frame: f32) -> Option<Transform> {
        if self.keyframes.is_empty() {
            return None;
        }

        if self.keyframes.len() == 1 || frame <= self.keyframes[0].frame as f32 {
            return Some(self.keyframes[0].transform);
        }

        let last_idx = self.keyframes.len() - 1;
        if frame >= self.keyframes[last_idx].frame as f32 {
            return Some(self.keyframes[last_idx].transform);
        }

        // Find the bracket [k0, k1] where k0.frame <= frame <= k1.frame
        let idx = self
            .keyframes
            .partition_point(|k| (k.frame as f32) < frame);
        let k1 = &self.keyframes[idx];
        let k0 = &self.keyframes[idx - 1];

        let span = (k1.frame - k0.frame) as f32;
        if span <= 1e-4 {
            return Some(k0.transform);
        }

        let mut t = ((frame - k0.frame as f32) / span).clamp(0.0, 1.0);

        match k0.interpolation {
            KeyframeInterpolation::Constant => {
                return Some(k0.transform);
            }
            KeyframeInterpolation::Bezier => {
                // Smooth cubic ease in/out (Hermite smoothstep)
                t = t * t * (3.0 - 2.0 * t);
            }
            KeyframeInterpolation::Linear => {}
        }

        let translation = k0.transform.translation.lerp(k1.transform.translation, t);
        let rotation = k0.transform.rotation.slerp(k1.transform.rotation, t);
        let scale = k0.transform.scale.lerp(k1.transform.scale, t);

        Some(Transform {
            translation,
            rotation,
            scale,
        })
    }

    /// Returns all frame indices that have keyframes in this track.
    pub fn keyframe_frames(&self) -> Vec<u32> {
        self.keyframes.iter().map(|k| k.frame).collect()
    }

    /// Finds the next keyframe strictly after `current_frame`.
    pub fn next_keyframe(&self, current_frame: u32) -> Option<u32> {
        self.keyframes
            .iter()
            .find(|k| k.frame > current_frame)
            .map(|k| k.frame)
    }

    /// Finds the previous keyframe strictly before `current_frame`.
    pub fn prev_keyframe(&self, current_frame: u32) -> Option<u32> {
        self.keyframes
            .iter()
            .rev()
            .find(|k| k.frame < current_frame)
            .map(|k| k.frame)
    }
}

/// Timeline settings and current playback state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineState {
    pub current_frame: u32,
    pub start_frame: u32,
    pub end_frame: u32,
    pub fps: u32,
    pub is_playing: bool,
    pub is_looping: bool,
    #[serde(skip)]
    pub accumulator: f32,
}

impl Default for TimelineState {
    fn default() -> Self {
        Self {
            current_frame: 0,
            start_frame: 0,
            end_frame: 100,
            fps: 24,
            is_playing: false,
            is_looping: true,
            accumulator: 0.0,
        }
    }
}

impl TimelineState {
    /// Advances playback by elapsed real time `dt_seconds`.
    /// Returns true if `current_frame` changed.
    pub fn advance_time(&mut self, dt_seconds: f32) -> bool {
        if !self.is_playing || self.fps == 0 {
            return false;
        }

        let frame_duration = 1.0 / (self.fps as f32);
        self.accumulator += dt_seconds;

        let frames_to_advance = (self.accumulator / frame_duration).floor() as u32;
        if frames_to_advance == 0 {
            return false;
        }

        self.accumulator -= (frames_to_advance as f32) * frame_duration;
        let old_frame = self.current_frame;

        let range = if self.end_frame > self.start_frame {
            self.end_frame - self.start_frame + 1
        } else {
            1
        };

        let new_frame = self.current_frame + frames_to_advance;
        if new_frame > self.end_frame {
            if self.is_looping {
                self.current_frame = self.start_frame + ((new_frame - self.start_frame) % range);
            } else {
                self.current_frame = self.end_frame;
                self.is_playing = false;
            }
        } else {
            self.current_frame = new_frame;
        }

        self.current_frame != old_frame
    }

    /// Moves the playhead by `delta` frames, clamping within bounds or looping.
    pub fn step_frame(&mut self, delta: i32) {
        let new_f = (self.current_frame as i32 + delta)
            .clamp(self.start_frame as i32, self.end_frame as i32);
        self.current_frame = new_f as u32;
        self.accumulator = 0.0;
    }

    /// Sets the playhead frame directly, clamped to start/end bounds.
    pub fn set_frame(&mut self, frame: u32) {
        self.current_frame = frame.clamp(self.start_frame, self.end_frame.max(self.start_frame));
        self.accumulator = 0.0;
    }

    /// Toggles playback on and off.
    pub fn toggle_playback(&mut self) -> bool {
        self.is_playing = !self.is_playing;
        self.accumulator = 0.0;
        self.is_playing
    }
}

/// Top-level animation system managing object tracks and timeline state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AnimationManager {
    pub tracks: HashMap<ObjectId, ObjectAnimationTrack>,
    pub timeline: TimelineState,
}

impl AnimationManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a keyframe on the given object at `frame`.
    pub fn insert_keyframe(&mut self, object_id: ObjectId, frame: u32, transform: Transform) {
        self.tracks
            .entry(object_id)
            .or_insert_with(|| ObjectAnimationTrack::new(object_id))
            .insert_keyframe(frame, transform);
    }

    /// Removes a keyframe from the given object at `frame`.
    pub fn remove_keyframe(&mut self, object_id: ObjectId, frame: u32) -> bool {
        if let Some(track) = self.tracks.get_mut(&object_id) {
            track.remove_keyframe(frame)
        } else {
            false
        }
    }

    /// Returns true if the object has a keyframe at `frame`.
    pub fn has_keyframe(&self, object_id: ObjectId, frame: u32) -> bool {
        self.tracks
            .get(&object_id)
            .map(|t| t.has_keyframe(frame))
            .unwrap_or(false)
    }

    /// Returns all keyframe frame indices for the given object.
    pub fn keyframe_frames(&self, object_id: ObjectId) -> Vec<u32> {
        self.tracks
            .get(&object_id)
            .map(|t| t.keyframe_frames())
            .unwrap_or_default()
    }

    /// Evaluates animated transform for a single object at `frame`.
    pub fn evaluate_object(&self, object_id: ObjectId, frame: f32) -> Option<Transform> {
        self.tracks.get(&object_id).and_then(|t| t.evaluate(frame))
    }

    /// Evaluates animated transforms for all tracked objects at `frame`.
    pub fn evaluate_all(&self, frame: f32) -> Vec<(ObjectId, Transform)> {
        self.tracks
            .iter()
            .filter_map(|(&id, track)| track.evaluate(frame).map(|tf| (id, tf)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::Vec3;

    #[test]
    fn test_keyframe_insertion_and_ordering() {
        let obj_id = ObjectId::new();
        let mut track = ObjectAnimationTrack::new(obj_id);

        let tf0 = Transform::from_translation(Vec3::new(0.0, 0.0, 0.0));
        let tf20 = Transform::from_translation(Vec3::new(10.0, 0.0, 0.0));
        let tf10 = Transform::from_translation(Vec3::new(5.0, 0.0, 0.0));

        track.insert_keyframe(20, tf20);
        track.insert_keyframe(0, tf0);
        track.insert_keyframe(10, tf10);

        let frames = track.keyframe_frames();
        assert_eq!(frames, vec![0, 10, 20]);
        assert!(track.has_keyframe(10));
        assert!(!track.has_keyframe(15));
    }

    #[test]
    fn test_track_linear_interpolation() {
        let obj_id = ObjectId::new();
        let mut track = ObjectAnimationTrack::new(obj_id);

        let tf0 = Transform::from_translation(Vec3::new(0.0, 0.0, 0.0));
        let tf10 = Transform::from_translation(Vec3::new(10.0, 20.0, 0.0));
        track.insert_keyframe(0, tf0);
        track.insert_keyframe(10, tf10);

        // Before start
        assert_eq!(track.evaluate(-5.0).unwrap().translation, Vec3::ZERO);
        // At start
        assert_eq!(track.evaluate(0.0).unwrap().translation, Vec3::ZERO);
        // Halfway
        let half = track.evaluate(5.0).unwrap().translation;
        assert!((half.x - 5.0).abs() < 1e-4);
        assert!((half.y - 10.0).abs() < 1e-4);
        // At end
        assert_eq!(track.evaluate(10.0).unwrap().translation, Vec3::new(10.0, 20.0, 0.0));
        // After end
        assert_eq!(track.evaluate(15.0).unwrap().translation, Vec3::new(10.0, 20.0, 0.0));
    }

    #[test]
    fn test_timeline_advance_and_loop() {
        let mut timeline = TimelineState {
            current_frame: 0,
            start_frame: 0,
            end_frame: 10,
            fps: 10,
            is_playing: true,
            is_looping: true,
            accumulator: 0.0,
        };

        // Advance 0.5s at 10 fps = 5 frames
        assert!(timeline.advance_time(0.5));
        assert_eq!(timeline.current_frame, 5);

        // Advance another 0.6s = 6 frames -> frame 11 -> loops back to frame 0
        assert!(timeline.advance_time(0.6));
        assert_eq!(timeline.current_frame, 0);
    }

    #[test]
    fn test_next_and_prev_keyframe_jump() {
        let obj_id = ObjectId::new();
        let mut track = ObjectAnimationTrack::new(obj_id);
        track.insert_keyframe(0, Transform::IDENTITY);
        track.insert_keyframe(25, Transform::IDENTITY);
        track.insert_keyframe(50, Transform::IDENTITY);

        assert_eq!(track.next_keyframe(0), Some(25));
        assert_eq!(track.next_keyframe(25), Some(50));
        assert_eq!(track.next_keyframe(50), None);

        assert_eq!(track.prev_keyframe(50), Some(25));
        assert_eq!(track.prev_keyframe(25), Some(0));
        assert_eq!(track.prev_keyframe(0), None);
    }
}
