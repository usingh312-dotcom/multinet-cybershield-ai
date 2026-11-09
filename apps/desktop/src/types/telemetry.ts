export type TrafficMetrics = {
  timestamp: string;
  interfaceId: string;
  downloadBytesPerSecond: number;
  uploadBytesPerSecond: number;
  totalDownloadBytes: number;
  totalUploadBytes: number;
};

export type ConnectionQuality = {
  latencyMs: number | null;
  jitterMs: number | null;
  packetLossPercent: number | null;
};

export type SpeedTestResult = {
  startedAt: string;
  completedAt: string | null;
  interfaceId: string | null;
  serverId: string | null;
  downloadMbps: number | null;
  uploadMbps: number | null;
  latencyMs: number | null;
  jitterMs: number | null;
  packetLossPercent: number | null;
  status: "queued" | "running" | "completed" | "failed" | "cancelled";
};

export type SpeedTestStageId = "prepare" | "latency" | "jitter" | "download" | "upload" | "report";
