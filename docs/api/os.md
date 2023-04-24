
# 系统 os

## Niva.api.os.info
```ts
/**
 * 获取系统信息，包括操作系统类型，体系结构和版本信息。
 * @returns 一个 Promise，在获取系统信息成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个包含操作系统信息的对象。
 */
export function info(): Promise<{
  os: string;
  arch: string;
  version: string;
}>;
```

## Niva.api.os.dirs

```ts
/**
 * 获取用户主目录以及与之相关的各种标准目录的路径。
 * @returns 一个 Promise，在获取目录信息成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个包含目录信息的对象。
 */
export function dirs(): Promise<{
  temp: string;
  data: string;
  home?: string;
  audio?: string;
  desktop?: string;
  document?: string;
  download?: string;
  font?: string;
  picture?: string;
  public?: string;
  template?: string;
  video?: string;
}>;
```

## Niva.api.os.sep
```ts
/**
 * 获取系统路径分隔符。
 * @returns 一个 Promise，在获取系统路径分隔符成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个字符串，表示系统路径分隔符。
 */
export function sep(): Promise<string>;
```

## Niva.api.os.eol

```ts
/**
 * 获取系统换行符。
 * @returns 一个 Promise，在获取系统换行符成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个字符串，表示系统换行符。
 */
export function eol(): Promise<string>;
```

## Niva.api.os.locale
```ts
/**
 * 获取系统区域设置，包括语言代码、国家/地区和编码方案。
 * @returns 一个 Promise，在获取区域设置信息成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个字符串，表示系统区域设置。
 */
export function locale(): Promise<string>;
```