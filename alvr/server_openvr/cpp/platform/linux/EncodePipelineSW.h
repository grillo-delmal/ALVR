#pragma once

#include "EncodePipeline.h"

class FormatConverter;

namespace alvr {

class EncodePipelineSW : public EncodePipeline {
public:
    ~EncodePipelineSW();
    EncodePipelineSW(Renderer* render, uint32_t width, uint32_t height);

    void PushFrame(uint64_t targetTimestampNs, bool idr) override;
    bool GetEncoded(FramePacket& packet) override;
    void SetParams(FfiDynamicEncoderParams params) override;
    int GetCodec() override;

};
}
