import { StateModel } from "@bramblex/state-model";
import { ComponentType } from "react";
import { uuid } from "../common/utils";
import { AppModel } from "./app.model";


export interface ModalComponentProps {
	close: () => any;
}

type ModalItem = {
	id: string,
	Component: ComponentType
}

export type ModalModelState = ModalItem[];

export class ModalModel extends StateModel<ModalModelState> {
	constructor(public readonly app: AppModel) {
		super([])
	}

	show<Props extends {}>(Component: ComponentType<Props & ModalComponentProps>, props: Props) {
		const id = uuid();
		const close = () => {
			this.setState(this.state.filter(({ id: _id }) => _id !== id));
		}
		this.setState(
			[...this.state, {
				id,
				Component: () => <Component {...props} close={close} />
			}]
		);
		return close;
	}

	async confirm(title: string, content: string): Promise<boolean> {
		return true
	}
}