# 托盘图标 tray

## Niva.api.tray.create
```ts
/**
 * 在系统托盘中创建一个新的托盘图标。
 * @param options 创建托盘图标的配置项。
 * @param window_id 要创建托盘图标的窗口 ID，默认为当前活动窗口 ID。
 * @returns 一个 Promise，在创建成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回新创建的托盘图标 ID。
 */
export function create(options: NivaTrayOptions, window_id?: number): Promise<number>;
```

## Niva.api.tray.destroy
```ts
/**
 * 销毁指定的托盘图标。
 * @param id 要销毁的托盘图标 ID。
 * @param window_id 要销毁托盘图标的窗口 ID，默认为当前活动窗口 ID。
 * @returns 一个 Promise，在销毁成功时解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function destroy(id: number, window_id?: number): Promise<void>;
```

## Niva.api.tray.destroyAll
```ts
/**
 * 销毁指定窗口的所有托盘图标。
 * @param window_id 要销毁托盘图标的窗口 ID，默认为当前活动窗口 ID。
 * @returns 一个 Promise，在销毁成功时解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function destroyAll(window_id?: number): Promise<void>;
```

## Niva.api.tray.list
```ts
/**
 * 获取指定窗口当前存在的所有托盘图标 ID。
 * @param window_id 要获取托盘图标 ID 的窗口 ID，默认为当前活动窗口 ID。
 * @returns 一个 Promise，在获取成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回托盘图标 ID 的数组。
 */
export function list(window_id?: number): Promise<number[]>;
```

## Niva.api.tray.update
```ts
/**
 * 更新指定托盘图标的配置项。
 * @param id 要更新的托盘图标 ID。
 * @param options 新的托盘图标配置项。
 * @param window_id 要更新托盘图标的窗口 ID，默认为当前活动窗口 ID。
 * @returns 一个 Promise，在更新成功时解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function update(id: number, options: NivaTrayUpdateOptions, window_id?: number): Promise<void>;
```