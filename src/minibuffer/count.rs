use bevy::{prelude::*, app::PluginGroupBuilder};
use bevy_minibuffer::{ui::IconContainer, prelude::*};
use std::marker::PhantomData;

pub struct CountComponentsActs {
    plugins: Option<PluginGroupBuilder>,
    acts: Acts,
}

#[derive(Component)]
struct CountText<C>(PhantomData<C>);

impl<C: Component> CountText<C> {
    fn new() -> Self {
        CountText(PhantomData)
    }
}

impl ActsPluginGroup for CountComponentsActs {
    fn acts(&self) -> &Acts {
        &self.acts
    }

    fn acts_mut(&mut self) -> &mut Acts {
        &mut self.acts
    }
}

impl Default for CountComponentsActs {
    fn default() -> Self {
        Self {
            plugins: Some(PluginGroupBuilder::start::<Self>()),
            acts: Acts::new([
                Act::new(show_count).bind(keyseq! { Space C }),
            ]),
        }
    }
}

impl PluginGroup for CountComponentsActs {
    fn build(mut self) -> PluginGroupBuilder {
        let builder = self.plugins.take().expect("plugin builder");
        builder.add(move |_app: &mut App| {
            // Add normal build() stuff here.
        })
    }
}

impl CountComponentsActs {
    pub fn add<C: Component>(mut self, name: impl Into<String>) -> Self {

    // pub fn add_with_name<C: Component>(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        let builder = self.plugins.take().expect("plugin group");
        self.plugins = Some(builder.add(move |app: &mut App| {
            let name = name.clone();
            app.add_systems(Startup, (move || name.clone()).pipe(setup_count::<C>))
                .add_systems(Update, update_count::<C>);
        }));
        self
    }
}

fn update_count<C: Component>(components: Query<Entity, With<C>>,
                              mut writer: TextUiWriter,
                              text: Single<Entity, With<CountText<C>>>) {
    let count = components.iter().count();
    *writer.text(*text, 2) = format!("{}", count);
}

fn setup_count<C: Component>(
    In(name): In<String>,
    icon_container: Single<Entity, With<IconContainer>>,
    mut commands: Commands,
) {
    commands.entity(*icon_container).with_children(|parent| {
        parent.spawn((Text::new(name),
                      CountText::<C>::new()))
            .with_children(|p| {
                p.spawn(TextSpan::new(" "));
                p.spawn(TextSpan::new("N/A"));
                p.spawn(TextSpan::new(" "));
            });
    });
}

pub fn show_count(_minibuffer: Minibuffer) {

}
