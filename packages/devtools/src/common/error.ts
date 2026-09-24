import type { AppModel } from "../models/app.model";

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

	SAVE_CONFIG_FAILED,
	SAVE_CONFIG_VALIDATE_FAILED,

  DEBUG_RESOURCE_NOT_FOUND,

  APP_EXIT_PREVENTED_BY_DIALOG,
  PROJECT_ALREADY_EXISTS,
  PACKAGER_BUILD_IN_PROGRESS,
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
      const { locale } = app.state;
      const messages: Partial<Record<ErrorCode, string>> = {
        [ErrorCode.PROJECT_PATH_NOT_EXISTS]: locale.t("ERR_PROJECT_PATH_NOT_EXISTS"),
        [ErrorCode.PROJECT_PATH_IS_NOT_DIR]: locale.t("ERR_PROJECT_PATH_IS_NOT_DIR"),
        [ErrorCode.PROJECT_CONFIG_NOT_EXISTS]: locale.t("ERR_PROJECT_CONFIG_NOT_EXISTS"),
        [ErrorCode.PROJECT_CONFIG_CRATE_FAILED]: locale.t("ERR_PROJECT_CONFIG_CREATE_FAILED"),
        [ErrorCode.PROJECT_CREATE_FAILED]: locale.t("ERR_PROJECT_CREATE_FAILED"),
        [ErrorCode.PROJECT_ALREADY_EXISTS]: locale.t("ERR_PROJECT_ALREADY_EXISTS"),
        [ErrorCode.PROJECT_LOAD_CONFIG_FAILED]: locale.t("ERR_PROJECT_LOAD_CONFIG_FAILED"),
        [ErrorCode.PROJECT_CONFIG_VALIDATE_FAILED]: locale.t("ERR_PROJECT_CONFIG_VALIDATE_FAILED"),
        [ErrorCode.SAVE_CONFIG_FAILED]: locale.t("ERR_SAVE_CONFIG_FAILED"),
        [ErrorCode.SAVE_CONFIG_VALIDATE_FAILED]: locale.t("ERR_SAVE_CONFIG_VALIDATE_FAILED"),
        [ErrorCode.DEBUG_RESOURCE_NOT_FOUND]: locale.t("ERR_DEBUG_RESOURCE_NOT_FOUND"),
        [ErrorCode.APP_EXIT_PREVENTED_BY_DIALOG]: locale.t("ERR_CLOSE_DIALOG_FIRST"),
        [ErrorCode.PACKAGER_BUILD_IN_PROGRESS]: locale.t("ERR_PACKAGER_BUILD_IN_PROGRESS"),
      };

      let message = messages[this.code] || locale.t("ERR_UNKNOWN");
      if (this.code === ErrorCode.UNKNOWN) {
        if (this.extra?.phase === "build") message = locale.t("ERR_BUILD_FAILED");
        if (this.extra?.phase === "sign") message = locale.t("ERR_SIGN_FAILED");
        if (this.extra?.phase === "open-output") message = locale.t("ERR_OPEN_OUTPUT_FAILED");
      }

      const detail = this.extra?.reason ?? this.extra?.error;
      const reason = detail instanceof Error ? detail.message : typeof detail === "string" ? detail : null;
      return reason ? `${message}\n${reason}` : message;
		}
}
