export type MonitorOverview = {
  generatedAt: string
  date: string
  summary: MonitorSummary
  members: MemberOverview[]
  codexKeys: CodexKeyOverview[]
  upstreams: UpstreamOverview[]
  recentEvents: UsageEventOverview[]
  config: ConfigOverview
}

export type MonitorSummary = {
  credits: number
  weightedTokens: number
  totalTokens: number
  requestCount: number
  messageCount: number
  wsConnectionCount: number
  memberCount: number
  activeMemberCount: number
  codexKeyCount: number
  activeCodexKeyCount: number
  upstreamKeyCount: number
  activeUpstreamKeyCount: number
  healthyUpstreamKeyCount: number
}

export type MemberOverview = {
  id: number
  name: string
  status: string
  weight: number
  maxConcurrentRequests: number
  currentConcurrentRequests: number
  fiveHourQuota: number
  weeklyQuota: number
  fiveHourUsage: WindowUsageOverview
  weeklyUsage: WindowUsageOverview
  credits: number
  weightedTokens: number
  totalTokens: number
  requestCount: number
  messageCount: number
  wsConnectionCount: number
  createdAt: string
}

export type WindowUsageOverview = {
  credits: number
  weightedTokens: number
  totalTokens: number
  requestCount: number
  messageCount: number
  wsConnectionCount: number
}

export type CodexKeyOverview = {
  id: number
  memberId: number
  memberName: string
  prefix: string
  status: string
  createdAt: string
  lastUsedAt: string | null
}

export type UpstreamOverview = {
  id: number
  name: string
  status: string
  supportedModels: string
  supportsHttp: boolean
  supportsWs: boolean
  weight: number
  maxConcurrentRequests: number
  currentConcurrentRequests: number
  failureCount: number
  cooldownUntil: string | null
  createdAt: string
  lastUsedAt: string | null
  healthy: boolean
}

export type UsageEventOverview = {
  id: number
  createdAt: string
  memberName: string
  codexKeyPrefix: string
  upstreamName: string | null
  protocol: string
  path: string
  model: string | null
  statusCode: number | null
  success: boolean
  promptTokens: number
  completionTokens: number
  totalTokens: number
  credits: number
  requestCount: number
  messageCount: number
  durationMs: number
  usagePrecision: string
  errorClass: string | null
}

export type ConfigOverview = {
  server: {
    bindAddr: string
  }
  upstream: {
    httpBaseUrl: string
    wsBaseUrl: string
    timeoutSecs: number
    retryAttempts: number
    cooldownSecs: number
    maxFailuresBeforeCooldown: number
  }
  credit: {
    accounting: string
    unknownModelMessageCredits: number
    unknownUsageCredits: number
  }
  limits: {
    defaultMemberConcurrency: number
    defaultMember5hQuota: number
    defaultMemberWeeklyQuota: number
    defaultUpstreamConcurrency: number
    wsIdleTimeoutSecs: number
    wsUpstreamPingIntervalSecs: number
    wsMaxConnectionSecs: number
    wsMaxMessagesPerConnection: number
  }
}

export async function fetchMonitorOverview(
  signal?: AbortSignal
): Promise<MonitorOverview> {
  const response = await fetch("/monitor/api/overview", {
    headers: {
      Accept: "application/json",
    },
    signal,
  })

  if (!response.ok) {
    throw new Error(await errorMessage(response))
  }

  return response.json()
}

async function errorMessage(response: Response) {
  try {
    const body = (await response.json()) as {
      error?: { message?: string; code?: string }
    }
    return body.error?.message ?? `${response.status} ${response.statusText}`
  } catch {
    return `${response.status} ${response.statusText}`
  }
}
