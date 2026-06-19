<!-- SPDX-FileCopyrightText: 2026 Denis Yermakou
SPDX-FileContributor: AxonOS
SPDX-License-Identifier: CC-BY-NC-ND-4.0 -->

# No Raw Neural Data

Neural Boundary Game v7.9.812 processes no real neural data.

- No EEG, ECoG, LFP, spike, or any other neural recording.
- No stimulation parameters, waveforms or hardware commands.
- No biometric data of any kind.
- No sensor hardware is connected or required.

The game entities (RawFrame, StimulationCommand, etc.) are abstract
constructs that illustrate boundary enforcement concepts. They contain
no actual signal data.

All game state is local. Nothing is transmitted to any server during play.
See `PRIVACY_NOTICE.md`.
