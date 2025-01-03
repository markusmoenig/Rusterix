import pickle
from enum import Enum

class EntityType(Enum):
    NPC = 1
    PLAYER = 2

class EntityAction(Enum):
    NONE = 0
    WEST = 1
    NORTH = 2
    EAST = 3
    SOUTH = 4

    def to_int(self):
        """Convert the Enum to its integer value."""
        return self.value

class EntityManager:
    def __init__(self, id):
        self.id = id
        self.entities = {}  # Dictionary to store entities by integer ID
        self.next_id = 0    # Counter for generating unique IDs

    def add_entity(self, entity):
        """Adds an entity and assigns it a unique integer ID."""
        if not isinstance(entity, Entity):
            raise TypeError("Only Entity instances can be added.")

        entity_id = self.next_id
        self.next_id += 1

        entity.id = entity_id
        entity.manager_id = self.id

        if entity.type == EntityType.PLAYER:
            register_player(self.id, entity_id)

        self.entities[entity_id] = entity
        return entity_id

    def event(self, entity_id, event, value):
        """Event"""
        self.entities[entity_id].event(event, value)

    def user_event(self, entity_id, event, value):
        """User based event"""
        self.entities[entity_id].user_event(event, value)

    def delete_entity(self, entity_id):
        """Deletes an entity by its ID."""
        if entity_id in self.entities:
            del self.entities[entity_id]
        else:
            raise KeyError(f"Entity with ID {entity_id} does not exist.")

    def get_entity_position(self, entity_id):
        """Gets the position of an entity."""
        if entity_id not in self.entities:
            raise KeyError(f"Entity with ID {entity_id} does not exist.")
        return self.entities[entity_id].position

    def set_entity_position(self, entity_id, position):
        """Sets the position of an entity."""
        if entity_id not in self.entities:
            raise KeyError(f"Entity with ID {entity_id} does not exist.")
        self.entities[entity_id].position = position

    def update_attribute(self, entity_id, key, value):
        """Updates or adds an attribute for a specific entity."""
        if entity_id not in self.entities:
            raise KeyError(f"Entity with ID {entity_id} does not exist.")
        self.entities[entity_id].update_attribute(key, value)

    def get_entity_attributes(self, entity_id):
        """Gets the dictionary of attributes for a specific entity."""
        if entity_id not in self.entities:
            raise KeyError(f"Entity with ID {entity_id} does not exist.")
        return self.entities[entity_id].get_all_attributes()

    def get_all_entities(self):
        """Returns all entities and their attributes."""
        return {eid: entity.get_all_attributes() for eid, entity in self.entities.items()}

    def serialize(self):
        """Serializes the EntityManager to a pickle string."""
        return pickle.dumps(self)

    @staticmethod
    def deserialize(data):
        """Deserializes an EntityManager from a pickle string."""
        return pickle.loads(data)

    def debug(self):
        """Prints all entities and their attributes."""
        print("Entities:")
        for entity_id, entity in self.entities.items():
            print(f" - ID {entity_id}: {entity.get_all_attributes()}")
