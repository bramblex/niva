import { StateModel } from '@bramblex/state-model'
import { useModel } from '@bramblex/state-model-react';
import classNames from 'classnames';
import style from './style.module.css'

interface MessageDialogProps {
	id: number,
	title: string,
	content: string,
	callback: (result: boolean) => void,
}

class DialogModel extends StateModel<MessageDialogProps[]> {
	private id = 0;

	constructor() {
		super([]);
	}

	message(title: string, content: string): Promise<boolean> {
		let resolve: ((result: boolean) => void) | null = null;
		const promise = new Promise<boolean>(r => resolve = r);
		let id = this.id++;
		this.setState([...this.state, {
			id,
			title, content,
			callback: (result) => {
				this.close(id);
				resolve?.(result)
			},
		}]);

		return promise
	}

	private close(id: number) {
		this.setState(this.state.filter(dialog => dialog.id !== id));
	}
}

export const dialog = new DialogModel();

export function DialogRoot() {
	const { state } = useModel(dialog);
	return <div className={style.dialogRoot}>
		{state.map((dialog, i) =>
			<div key={dialog.id} className={classNames(style.dialogContainer, {
				[style.dialogContainerActive]: i === state.length - 1,
			})}>
				<MessageDialog key={dialog.id} {...dialog} />
			</div>
		)}
	</div>
}

export function MessageDialog(props: MessageDialogProps) {
	return <div className={style.messageDialog}>
		<div className={style.messageDialogTitle}>{props.title}</div>
		<div className={style.messageDialogContent}>{props.content}</div>
		<div className={style.messageDialogButtonGroup}>
			<div className={style.messageDialogButton} onClick={() => props.callback(false)}>取消</div>
			<div className={classNames(style.messageDialogButton, style.messageDialogButtonPrimary)} onClick={() => props.callback(true)}>确定</div>
		</div>
	</div>
}

