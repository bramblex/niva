
import { StateModel } from '@bramblex/state-model'
import { useModel } from '@bramblex/state-model-react'
import classNames from 'classnames';
import { ComponentType } from 'react';
import { createPromise, uuid } from './utils';

export class ProgressModel extends StateModel<{ text: string, progress: number, error: boolean }> {
	private tasks: [string, () => Promise<void>][] = [];

	constructor() {
		super({ text: '', progress: 0, error: false });
	}

	addTask(text: string, task: () => Promise<void>) {
		this.tasks.push([text, task]);
	}

	async run() {
		for (let i = 0, l = this.tasks.length; i < l; i++) {
			const [text, task] = this.tasks[i];
			this.setState({
				...this.state,
				text: `(${i + 1}/${l})${text}`,
				progress: i / l,
				error: false
			});
			try {
				await task();
				await new Promise(resolve => setTimeout(resolve, 100));
			} catch (e) {
				this.setState({ ...this.state, text: (e as any).toString(), error: true });
				throw e;
			}
		}
		this.setState({ ...this.state, progress: 1 });
		await new Promise(resolve => setTimeout(resolve, 300));
	}
}

export type DialogComponentProps = { close: () => any };

class ModelModel extends StateModel<[string, ComponentType][]> {
	constructor() {
		super([]);
	}

	show<Props extends {}>(Component: ComponentType<Props & DialogComponentProps>, props: Props) {
		const id = uuid();
		const close = () => {
			this.setState(this.state.filter(([_id]) => _id !== id));
		}
		this.setState(
			[...this.state, [id, () => <Component close={close} {...props} />]]
		);
		return close;
	}


	alert(title: string, message: string): Promise<void> {
		const promise = createPromise<void>();

		function Alert({ close }: DialogComponentProps) {
			return <div className="window active is-bright">
				<div className="title-bar">
					<div className="title-bar-text" id="dialog-title">{title}</div>
					<div className="title-bar-controls">
						<button aria-label="Close" onClick={() => {
							close();
							promise.resolve();
						}}></button>
					</div>
				</div>
				<div className="window-body has-space">
					<p>{message}</p>
				</div>
				<footer style={{ textAlign: "right" }}>
					<button onClick={() => {
						close();
						promise.resolve();
					}}>确认</button>
				</footer>
			</div>
		}

		this.show(Alert, {})
		return promise;
	}

	confirm(title: string, message: string): Promise<boolean> {
		const promise = createPromise<boolean>();

		function Confirm({ close }: DialogComponentProps) {
			return <div className="window active is-bright">
				<div className="title-bar">
					<div className="title-bar-text" id="dialog-title">{title}</div>
					<div className="title-bar-controls">
						<button aria-label="Close" onClick={() => {
							close();
							promise.resolve(false);
						}}></button>
					</div>
				</div>
				<div className="window-body has-space">
					<p>{message}</p>
				</div>
				<footer style={{ textAlign: "right" }}>
					<button style={{ marginRight: '6px' }} onClick={() => {
						close();
						promise.resolve(false);
					}}>取消</button>
					<button className="default" onClick={() => {
						close();
						promise.resolve(true);
					}}>确认</button>
				</footer>
			</div>
		}

		this.show(Confirm, {})
		return promise;
	}

	progress(title: string, message: string): [ProgressModel, () => void] {
		const progress = new ProgressModel();
		function Progress({ close }: DialogComponentProps) {
			useModel(progress);
			const { state } = progress;

			return (
				<div className="window active is-bright">
					<div className="title-bar">
						<div className="title-bar-text" id="dialog-title">{title}</div>
					</div>
					<div className="window-body has-space">
						<h2 className="instruction instruction-primary">{message}</h2>

						{state.progress > 0 ?
							<div role={classNames("progressbar", { error: state.error })} className="animate">
								<div
									style={{ width: state.progress * 100 + '%' }}
									className="progressbar-animated"
								></div>
							</div>
							: <div role="progressbar" className="marquee"></div>}


						<p>{state.text}</p>
					</div>
					{state.error ? <footer style={{ textAlign: "right" }}>
						<button className="default" onClick={() => {
							close();
						}}>确认</button>
					</footer> : null}
				</div>
			)
		}
		return [progress, this.show(Progress, {})];
	}
}

export const modal = new ModelModel();

export function Modal() {
	const { state: modals } = useModel(modal);

	return modals.length > 0 ? (<div className='modal-container'>
		{modals.map(([id, Component], i) =>
			<div key={id} className={classNames('modal', { 'modal-active': modals.length === i + 1 })}><Component /></div>)}
	</div>) : null
}