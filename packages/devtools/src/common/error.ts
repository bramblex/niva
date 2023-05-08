import { AppModel } from "../models/app.model";

export enum ErrorCode {
	UNKNOWN,

	PROJECT_PATH_NOT_EXISTS,
	PROJECT_PATH_IS_NOT_DIR,
	PROJECT_CONFIG_NOT_EXISTS,
	PROJECT_CONFIG_CRATE_FAILED,
	PROJECT_CREATE_FAILED,

	PROJECT_LOAD_CONFIG_FAILED,
	PROJECT_CONFIG_VALIDATE_FAILED,
	PROJECT_HAS_UNSAVED_CHANGE,

	APP_EXIT_PREVENTED_BY_DIALOG,
}

export class AppError extends Error {
	code: ErrorCode;
	extra?: Record<string, any>;

	constructor(code: ErrorCode, extra?: Record<string, any>) {
		super(`[${ErrorCode[code]}] code: ${code}, extra: ${JSON.stringify(extra)}`)
		this.code = code;
		this.extra = extra;
	}

	toLocaleMessage(app: AppModel): string {
		// const { locale } = app.state;
		// @TODO: 补全本地化逻辑逻辑
		return this.message;
	}
}
