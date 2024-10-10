#include "EncodePipelineSW.h"

#include "FormatConverter.h"
#include "alvr_server/Logger.h"
#include "alvr_server/Settings.h"

alvr::EncodePipelineSW::EncodePipelineSW(Renderer* render, uint32_t width, uint32_t height) {
    throw std::runtime_error("No sw encoder implemented");
}

alvr::EncodePipelineSW::~EncodePipelineSW() {
}

void alvr::EncodePipelineSW::PushFrame(uint64_t targetTimestampNs, bool idr) {
    throw std::runtime_error("openh264 EncodeFrame failed");
}

bool alvr::EncodePipelineSW::GetEncoded(FramePacket& packet) {
    return false;
}

void alvr::EncodePipelineSW::SetParams(FfiDynamicEncoderParams params) {
}

int alvr::EncodePipelineSW::GetCodec() { return ALVR_CODEC_H264; }
