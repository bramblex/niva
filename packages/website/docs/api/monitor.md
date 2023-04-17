# 监视器 monitor

## Niva.api.monitor.list
```ts
/**
 * 列出系统中可用的所有监视器，并返回它们的信息。包括每个监视器的名称、大小、位置、物理大小、物理位置和缩放因子。
 * @returns 一个 Promise，在获取监视器信息成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个包含所有监视器信息的数组。
 */
export function list(): Promise<{
    name: string;
    size: { width: number; height: number };
    position: { x: number; y: number };
    physicalSize: { width: number; height: number };
    physicalPosition: { x: number; y: number };
    scaleFactor: number;
}[]>;
```

## Niva.api.monitor.current
```ts
/**
 * 获取包含当前窗口的监视器的信息，包括该监视器的名称、大小、位置、物理大小、物理位置和缩放因子。
 * @returns 一个 Promise，在获取监视器信息成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回包含监视器信息的对象。
 */
export function current(): Promise<{
    name: string;
    size: { width: number; height: number };
    position: { x: number; y: number };
    physicalSize: { width: number; height: number };
    physicalPosition: { x: number; y: number };
    scaleFactor: number;
} | null>;
```

## Niva.api.monitor.primary
```ts
/**
 * 获取系统中主监视器的信息，包括该监视器的名称、大小、位置、物理大小、物理位置和缩放因子。
 * @returns 一个 Promise，在获取主监视器信息成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回包含监视器信息的对象。
 */
export function primary(): Promise<{
    name: string;
    size: { width: number; height: number };
    position: { x: number; y: number };
    physicalSize: { width: number; height: number };
    physicalPosition: { x: number; y: number };
    scaleFactor: number;
} | null>;
```

## Niva.api.monitor.fromPoint
```ts
/**
 * 获取指定坐标点所在的监视器的信息，包括该监视器的名称、大小、位置、物理大小、物理位置和缩放因子。
 * @param x 坐标点的 X 坐标。
 * @param y 坐标点的 Y 坐标。
 * @returns 一个 Promise，在获取监视器信息成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回包含监视器信息的对象。
 */
export function fromPoint(x: number, y: number): Promise<{
    name: string;
    size: { width: number; height: number };
    position: { x: number; y: number };
    physicalSize: { width: number; height: number };
    physicalPosition: { x: number; y: number };
    scaleFactor: number;
} | null>;
```