import * as neverThrow from 'neverthrow';
import { AppError, ErrorCode } from './error';

export type AppResult<T = void> = neverThrow.Result<T, AppError>;

export function Ok<T>(value: T): AppResult<T> {
	if (value === undefined) {
		return neverThrow.ok(void 0) as AppResult<T>;
	}
	return neverThrow.ok(value);
}

export function Err(code: ErrorCode, extra?: Record<string, any>): AppResult<any> {
	return neverThrow.err(new AppError(code, extra))
}

export function fromThrowable<T>(p: () => T): AppResult<T> {
	try {
		return Ok(p());
	} catch (e) {
		return Err(ErrorCode.UNKNOWN, { error: e });
	}
}

export function fromThrowableAsync<T>(p: () => Promise<T>): Promise<AppResult<T>> {
	return p().then(Ok).catch((e) => Err(ErrorCode.UNKNOWN, { error: e }));
}